use crate::{
    process::{
        KERNEL_PROCESS,
        context::{ArchTaskContext, TaskContext},
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
    /// 如果该任务为一个内核线程，则该字段为 Some，表示内核线程的入口
    pub kthread_entry: Option<fn() -> !>,
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

    pub fn new_kthread(entry: fn() -> !) -> Arc<Self> {
        let kstack = KernelStackAllocator::alloc();
        let (_, kstack_top) = kstack.range();
        let process = KERNEL_PROCESS.clone();
        let mut process_inner = process.lock();
        let id = process_inner.avail_task_id.alloc();
        let task = Arc::new(Self {
            process: Arc::downgrade(&process),
            kstack,
            id,
            kthread_entry: Some(entry),
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
