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
    /// 内核函数的入口会先被**暂存**到这里，当前 task 被首次调度时，会把这里的闭包
    /// **所有权**取出。这意味着不能通过判断 kthread_entry 是否为 Some 来判断该
    /// task 是否为内核线程，内核线程在第一次被调度后，这里永远是 None
    pub kthread_entry: KthreadEntryCell,
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
