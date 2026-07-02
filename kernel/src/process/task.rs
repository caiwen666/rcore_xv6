use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::{PhysAddr, VirtAddr},
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission},
    },
    process::{
        KERNEL_PROCESS, KERNEL_PROCESS_RESOURCE, ProcessControlBlock, ProcessResource,
        ProcessStatus,
        context::{
            ArchTaskContext, ArchTrapContext, TRAP_CONTEXT_PAGE_COUNT, TaskContext, TrapContext,
        },
        kthread::KthreadEntryCell,
        mm::{KernelStack, KernelStackAllocator, USER_HEAP_START},
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
};
use alloc::sync::Arc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum TaskStatus {
    /// 准备运行
    Ready,
    /// 正在运行
    Running,
    /// 正在堵塞
    Blocked,
}

pub struct TaskControlBlock {
    process: Arc<ProcessControlBlock>,
    process_resource: Arc<SpinMutex<ProcessResource>>,
    #[expect(unused)]
    kstack: KernelStack,
    pub id: usize,
    /// 内核线程的入口函数
    ///
    /// 入口闭包会先被**暂存**到这里；当前 task 首次进入 `task_entry` 时，会通过
    /// `take()` 将闭包的**所有权**取出并执行。取出后 cell 内部会变为 `None`，因此
    /// 不能通过 `take()` 的返回值是否为 `Some` 来长期判断该 task 是否为内核线程。
    pub(super) kthread_entry: KthreadEntryCell,
    inner: SpinMutex<TaskControlBlockInner>,
    /// 内核线程没有 trap 上下文
    trap_context_paddr: Option<PhysAddr>,
}

pub struct TaskControlBlockInner {
    pub status: TaskStatus,
    pub task_context: ArchTaskContext,
}

impl TaskControlBlock {
    pub fn lock(&self) -> SpinMutexGuard<'_, TaskControlBlockInner> {
        self.inner.lock()
    }

    pub unsafe fn unlock(&self) {
        unsafe { self.inner.unlock() };
    }

    pub fn process(&self) -> Arc<ProcessControlBlock> {
        self.process.clone()
    }

    pub fn process_resource(&self) -> Arc<SpinMutex<ProcessResource>> {
        self.process_resource.clone()
    }

    pub fn trap_context(&self) -> &mut ArchTrapContext {
        self.trap_context_paddr.unwrap().get_mut()
    }
}

impl TaskControlBlock {
    pub fn new_kthread(entry: KthreadEntryCell) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_top) = kstack.range();
        let mut resource = KERNEL_PROCESS_RESOURCE.lock();
        let id = resource.avail_task_id.alloc();
        let task = Arc::new(Self {
            process_resource: KERNEL_PROCESS_RESOURCE.clone(),
            process: KERNEL_PROCESS.clone(),
            kstack,
            id,
            kthread_entry: entry,
            inner: SpinMutex::new(
                TaskControlBlockInner {
                    status: TaskStatus::Ready,
                    task_context: ArchTaskContext::new(kstack_top),
                },
                "task_control_block_inner",
            ),
            trap_context_paddr: None,
        });
        if id >= resource.tasks.len() {
            resource.tasks.push(Some(Arc::downgrade(&task)));
        } else {
            resource.tasks[id] = Some(Arc::downgrade(&task));
        }
        task
    }

    /// 在指定进程下创建线程。线程的默认状态为 [TaskStatus::Ready]。不会自动将线程加入调度器。
    pub fn new(
        process: Arc<ProcessControlBlock>,
        process_resource: Arc<SpinMutex<ProcessResource>>,
        entry: VirtAddr,
    ) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_high) = kstack.range();

        let mut resource = process_resource.lock();
        let id = resource.avail_task_id.alloc();
        // 分配用户栈
        let (ustack_low, ustack_high) = process.ustack_vaddr(id);
        let memory_space = resource.memory_space.as_mut().unwrap();
        memory_space.push(MemoryArea::new(
            ustack_low,
            ustack_high - ustack_low,
            MemoryPermission::Readable
                | MemoryPermission::Writable
                | MemoryPermission::UserAccessible,
            MemoryAreaType::Private,
            "ustack",
        ));
        // 分配 trap 上下文
        memory_space.push(MemoryArea::new(
            process.trap_context_vaddr(id),
            TRAP_CONTEXT_PAGE_COUNT * MMArch::PAGE_SIZE,
            // 不给 U 权限
            MemoryPermission::Readable | MemoryPermission::Writable,
            MemoryAreaType::Private,
            "trap_context",
        ));
        // 分配堆
        memory_space.push(MemoryArea::new(
            VirtAddr::new(USER_HEAP_START),
            0,
            MemoryPermission::Readable
                | MemoryPermission::Writable
                | MemoryPermission::UserAccessible,
            MemoryAreaType::Private,
            "heap",
        ));
        // 分配 tls
        let mut tls_base = VirtAddr::new(0);
        if let Some(tls_size) = process.tls_size {
            tls_base = process.tls_vaddr(id).unwrap();
            memory_space.push(MemoryArea::new(
                tls_base,
                tls_size,
                MemoryPermission::Readable
                    | MemoryPermission::Writable
                    | MemoryPermission::UserAccessible,
                MemoryAreaType::Private,
                "tls",
            ));
        }
        // 拿到 trap 上下文的物理地址
        let (trap_context_paddr, _) = memory_space
            .translate_vaddr(process.trap_context_vaddr(id))
            .unwrap();

        let task = Arc::new(Self {
            process,
            process_resource: process_resource.clone(),
            kstack,
            id,
            kthread_entry: KthreadEntryCell::empty(),
            inner: SpinMutex::new(
                TaskControlBlockInner {
                    status: TaskStatus::Ready,
                    task_context: ArchTaskContext::new(kstack_high),
                },
                "task_control_block_inner",
            ),
            trap_context_paddr: Some(trap_context_paddr),
        });

        // 写入 trap 上下文
        *task.trap_context() = ArchTrapContext::new(kstack_high)
            .set_ustack(ustack_high)
            .set_pc(entry)
            .set_tls_base(tls_base);

        if id >= resource.tasks.len() {
            resource.tasks.push(Some(Arc::downgrade(&task)));
        } else {
            resource.tasks[id] = Some(Arc::downgrade(&task));
        }

        task
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        // 归还 tid
        let mut resource = self.process_resource.lock();
        resource.avail_task_id.dealloc(self.id);
        resource.tasks[self.id] = None;
        // 是否为最后一个线程，是的话就去标记这个进程已经结束
        // 内核进程除外
        if resource.avail_task_id.count() == 0 && self.process.pid != 0 {
            let mut process_inner = self.process.inner();
            process_inner.status = ProcessStatus::Zombie(0);
        }
    }
}
