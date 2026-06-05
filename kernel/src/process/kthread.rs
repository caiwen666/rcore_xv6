use core::cell::UnsafeCell;

use alloc::boxed::Box;

use crate::process::{cpu::CPUManager, schedule::TaskScheduler, task::TaskControlBlock};

pub struct KthreadEntryCell {
    inner: UnsafeCell<Option<Box<dyn FnOnce() -> ! + Send + 'static>>>,
}

// SAFETY: KthreadEntryCell 只有在 task_entry 中被访问，而一个 task 只会进一次 task_entry
unsafe impl Sync for KthreadEntryCell {}

impl KthreadEntryCell {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce() -> ! + Send + 'static,
    {
        Self {
            inner: UnsafeCell::new(Some(Box::new(f))),
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
    F: FnOnce() -> ! + Send + 'static,
{
    let task = TaskControlBlock::new_kthread(KthreadEntryCell::new(f));
    TaskScheduler::push(task);
}

/// 退出当前的内核线程
pub fn exit_kthread() -> ! {
    let task = CPUManager::current_task().expect("exit_kthread: current_task is None");
    let mut task_inner = task.lock();
    // 内核线程目前没什么要回收的资源

    // 内核线程也不用变成僵尸线程来等待其他的线程来回收
    let process = task.process.upgrade().unwrap();
    assert!(
        process.pid == 0,
        "exit_kthread: kthread should be in kernel process"
    );
    let mut process_inner = process.lock();
    process_inner.avail_task_id.dealloc(task.id);
    process_inner.tasks[task.id] = None;
    drop(process_inner);
    drop(process);

    // SAFETY: 当前正在对 task_inner 加锁，所以中断还是关闭的
    let cpu = unsafe { CPUManager::current_cpu() };
    // 返回调度循环，该任务不会再被调度了
    let current_context = &mut task_inner.context as *mut _;
    // 下面行为的原因见 CPUManager::yield_current_task 的注释
    // SAFETY: 调度循环会进行解锁
    unsafe { task_inner.leak() };
    drop(task_inner);
    drop(task);
    cpu.go_scheduler(current_context);
    unreachable!()
}
