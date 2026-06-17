use crate::process::{cpu::CPUManager, schedule::TaskScheduler, task::TaskControlBlock};
use alloc::boxed::Box;
use core::cell::UnsafeCell;

pub struct KthreadEntryCell {
    inner: UnsafeCell<Option<Box<dyn FnOnce() -> ! + Send + 'static>>>,
}

// SAFETY: KthreadEntryCell 只有在 task_entry 中被访问，而一个 task 只会进一次 task_entry
unsafe impl Sync for KthreadEntryCell {}

impl KthreadEntryCell {
    pub fn empty() -> Self {
        Self {
            inner: UnsafeCell::new(None),
        }
    }

    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let f_with_exit = move || {
            f();
            CPUManager::exit_current_task();
        };
        Self {
            inner: UnsafeCell::new(Some(Box::new(f_with_exit))),
        }
    }

    /// # Safety
    ///
    /// 该函数必须在 [crate::process::schedule::task_entry] 中调用，且只调用一次。
    pub unsafe fn take(&self) -> Option<Box<dyn FnOnce() -> ! + Send + 'static>> {
        unsafe { (*self.inner.get()).take() }
    }
}

pub fn spawn_kthread<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    let task = TaskControlBlock::new_kthread(KthreadEntryCell::new(f));
    TaskScheduler::push(task);
}
