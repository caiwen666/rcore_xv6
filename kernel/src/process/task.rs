use crate::{
    process::{
        KERNEL_PROCESS,
        context::{ArchTaskContext, TaskContext},
        kthread::KthreadEntryCell,
        mm::{KernelStack, KernelStackAllocator},
        process::ProcessControlBlock,
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
};
use alloc::sync::{Arc, Weak};

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
    pub process: Weak<ProcessControlBlock>,
    #[expect(unused)]
    pub kstack: KernelStack,
    pub id: usize,
    /// 内核线程的入口函数
    ///
    /// 入口闭包会先被**暂存**到这里；当前 task 首次进入 `task_entry` 时，会通过
    /// `take()` 将闭包的**所有权**取出并执行。取出后 cell 内部会变为 `None`，因此
    /// 不能通过 `take()` 的返回值是否为 `Some` 来长期判断该 task 是否为内核线程。
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

    pub fn new_kthread(entry: KthreadEntryCell) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_top) = kstack.range();
        let process = KERNEL_PROCESS.clone();
        let mut process_inner = process.lock();
        let id = process_inner.avail_task_id.alloc();
        let task = Arc::new(Self {
            process: Arc::downgrade(&process),
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
        if id >= process_inner.tasks.len() {
            process_inner.tasks.push(Some(task.clone()));
        } else {
            process_inner.tasks[id] = Some(task.clone());
        }
        task
    }
}
