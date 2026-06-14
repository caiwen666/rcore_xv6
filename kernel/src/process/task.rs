use crate::{
    process::{
        KERNEL_PROCESS, KERNEL_PROCESS_RESOURCE, ProcessControlBlock, ProcessResource,
        ProcessStatus,
        context::{ArchTaskContext, TaskContext},
        kthread::KthreadEntryCell,
        mm::{KernelStack, KernelStackAllocator},
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
};
use alloc::sync::Arc;

#[derive(Copy, Clone, PartialEq)]
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
}

pub struct TaskControlBlockInner {
    pub status: TaskStatus,
    pub context: ArchTaskContext,
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
                    context: ArchTaskContext::new(kstack_top),
                },
                "task_control_block_inner",
            ),
        });
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
