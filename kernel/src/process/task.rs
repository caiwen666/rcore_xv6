use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::{PhysAddr, VirtAddr},
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission},
    },
    process::{
        KERNEL_PROCESS, ProcessControlBlock, ProcessStatus,
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
    /// 正在堵塞，里面的 bool 表示是否可中断，true 为可中断
    Blocked(bool),
}

/// TaskControlBlock(TCB)
///
/// TCB 放在堆上面，其强引用只会出现在如下几个地方：
///
/// - 正在运行的 task 的栈上
/// - 调度队列
/// - CPU
///
/// 其他地方应保存 TCB 的弱引用。
///
/// 这确保任务只要还活着就必然存在一个强引用指向它。强引用归零时，任务必然结束并被回收。
pub struct TaskControlBlock {
    process: Arc<ProcessControlBlock>,
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
    /// 是否收到了 kill 信号。
    /// 收到该信号时，如果当前状态为 Block(true) 的话，则立刻唤醒当前线程，
    /// 当前线程应立刻完成清理工作并退出。
    /// 如果当前状态为 Block(false) 的话，线程会在完成系统调用之后，返回用户态之前退出。
    ///
    /// killed 信号一旦被加上就不可被撤销。
    pub killed: bool,
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

    pub fn trap_context(&self) -> &mut ArchTrapContext {
        self.trap_context_paddr.unwrap().get_mut()
    }
}

impl TaskControlBlock {
    pub fn new_kthread(entry: KthreadEntryCell) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_top) = kstack.range();
        let mut inner = KERNEL_PROCESS.inner();
        let id = inner.tasks.next_id();
        let task = Arc::new(Self {
            process: KERNEL_PROCESS.clone(),
            kstack,
            id,
            kthread_entry: entry,
            inner: SpinMutex::new(
                TaskControlBlockInner {
                    status: TaskStatus::Ready,
                    task_context: ArchTaskContext::new(kstack_top),
                    killed: false,
                },
                "task_control_block_inner",
            ),
            trap_context_paddr: None,
        });
        inner.tasks.push(Arc::downgrade(&task));
        task
    }

    /// 在指定进程下创建线程。线程的默认状态为 [TaskStatus::Ready]。不会自动将线程加入调度器。
    pub fn new(process: Arc<ProcessControlBlock>, entry: VirtAddr) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_high) = kstack.range();

        let mut inner = process.inner();
        let id = inner.tasks.next_id();
        // 分配用户栈
        let (ustack_low, ustack_high) = process.ustack_vaddr(id);
        let memory_space = inner.memory_space.as_mut().unwrap();
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
            process: process.clone(),
            kstack,
            id,
            kthread_entry: KthreadEntryCell::empty(),
            inner: SpinMutex::new(
                TaskControlBlockInner {
                    status: TaskStatus::Ready,
                    task_context: ArchTaskContext::new(kstack_high),
                    killed: false,
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

        inner.tasks.push(Arc::downgrade(&task));

        task
    }

    /// fork 当前线程，并绑到指定进程上。不会将线程放入调度循环中。
    pub(super) fn fork(&self, process: Arc<ProcessControlBlock>) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_high) = kstack.range();
        let mut process_inner = process.inner();
        let memory_space = process_inner.memory_space.as_ref().unwrap();
        let (trap_context_paddr, _) = memory_space
            .translate_vaddr(process.trap_context_vaddr(self.id))
            .unwrap();
        let task = Arc::new(Self {
            process: process.clone(),
            kstack,
            id: self.id,
            kthread_entry: KthreadEntryCell::empty(),
            inner: SpinMutex::new(
                TaskControlBlockInner {
                    status: TaskStatus::Ready,
                    task_context: ArchTaskContext::new(kstack_high),
                    killed: false,
                },
                "task_control_block_inner",
            ),
            trap_context_paddr: Some(trap_context_paddr),
        });
        // trap 上下文页从父进程复制而来，其中的 kernel_sp 仍指向父进程内核栈，
        // 子进程陷入内核时必须使用自己的内核栈。
        task.trap_context().kernel_sp = kstack_high.inner();
        process_inner.tasks.insert(self.id, Arc::downgrade(&task));
        task
    }
}

impl Drop for TaskControlBlock {
    // 最后一个引用计数应该是在调度循环中归零的。调用 drop 时不持有任何锁。
    fn drop(&mut self) {
        // 归还 tid
        let mut inner = self.process.inner();
        inner.tasks.pop(self.id);
        if inner.tasks.len() == 0 && self.process.pid != KERNEL_PROCESS.pid {
            // 最后一个线程负责退出整个进程（内核线程除外）
            let code = match inner.status {
                ProcessStatus::Exiting(code) => code,
                _ => 0,
            };
            // 将子进程指向当前进程的 PCB 引用和当前进程指向子进程的引用砍掉
            for child in inner.children.values() {
                let mut child_inner = child.inner();
                child_inner.parent = None;
            }
            inner.children.clear();
            // 对父进程加锁时要格外小心，因为容易出现子进程持有自己的锁，要对父进程加锁，
            // 而父进程也持有自己的锁，要对子进程加锁，这就形成了死锁
            // 我们统一按先父后子的顺序进行加锁
            // 这里先取当前子进程的父进程PCB，同时将当前进程到父进程的引用砍掉
            let Some(origin_parent) = inner.parent.take() else {
                // 说明当前进程已经是孤儿进程了
                return;
            };
            drop(inner);
            // 再分别对父进程和子进程加锁，锁序转变为先父后子
            let mut parent_inner = origin_parent.inner();
            // 从 drop(inner) 到这里再对 inner 加锁，这个窗口，有可能 origin_parent 已经不是当前进程的父进程了
            // 所以我们需要复查一下
            let mut inner = self.process.inner();
            if !parent_inner.children.contains_key(&self.process.pid) {
                // 说明当前进程已经不是父进程的子进程了
                // 同时也说明当前进程是一个孤儿进程了
                return;
            }
            parent_inner.exited_children.insert(self.process.pid, code);
            // 砍掉当前进程到父进程和父进程到当前进程的引用
            parent_inner.children.remove(&self.process.pid);
            inner.parent = None;
        }
    }
}
