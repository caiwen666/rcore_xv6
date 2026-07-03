use crate::{
    arch::IrqArch,
    exception::InterruptArch,
    process::{
        ProcessManager,
        cpu::CPUManager,
        task::{TaskControlBlock, TaskControlBlockInner, TaskStatus},
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
};
use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use lazy_static::lazy_static;

pub struct TaskScheduler {
    pub(self) queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskScheduler {
    pub(self) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

lazy_static! {
    static ref TASK_SCHEDULER: SpinMutex<TaskScheduler> =
        SpinMutex::new(TaskScheduler::new(), "task_scheduler");
}

impl TaskScheduler {
    /// 将某个任务放入调度器中
    pub fn push(task: Arc<TaskControlBlock>) {
        TASK_SCHEDULER.lock().queue.push_back(task);
    }
    /// 从调度器中获取一个任务
    pub fn pop() -> Option<Arc<TaskControlBlock>> {
        TASK_SCHEDULER.lock().queue.pop_front()
    }
}

/// 调度循环
///
/// # Safety
///
/// 调用时需要关闭中断
pub unsafe fn schedule_loop() -> ! {
    // SAFETY: 调用者已经保证了中断是关闭的
    let cpu = unsafe { CPUManager::current_cpu() };
    loop {
        IrqArch::enable_interrupt();
        if let Some(task) = TaskScheduler::pop() {
            let mut task_inner = task.lock();
            cpu.current_task = Some(task.clone());
            task_inner.status = TaskStatus::Running;
            // 不用释放 task_inner
            // 如果这里的切换回到了 ProcessManager::go_scheduler，
            // 其要求必须是拿住线程的锁的，所以 ProcessManager::go_scheduler 的调用者会释放
            // 如果这里的切换去到了 task_entry，这个函数最开始也会把锁释放掉
            unsafe {
                // SAFETY: 当前持有了 task_inner 的锁，所以中断是关闭的
                IrqArch::switch_context(
                    &mut cpu.idle_task_context as *mut _,
                    &mut task_inner.task_context as *mut _,
                );
            }
            // 到这里说明任务调度完毕了
            cpu.current_task = None;
        }
    }
}

/// 任务第一次被调度时会先执行这里，此时中断还是关闭的
pub fn task_entry() {
    // SAFETY: 刚从调度循环过来，中断还是关闭的
    let cpu = unsafe { CPUManager::current_cpu() };
    let task = cpu
        .current_task
        .clone()
        .expect("task_entry: current_task is None");
    unsafe { task.unlock() };
    if let Some(kthread_entry) = unsafe { task.kthread_entry.take() } {
        // 进入 kthread_entry 之后就不再返回了，所以需要提前把 task 释放掉
        drop(task);
        IrqArch::enable_interrupt();
        kthread_entry();
    }
    drop(task);
    IrqArch::return_to_user();
}

impl ProcessManager {
    /// 将当前上下文保存，并回到调度循环中
    ///
    /// **WARNING**：注意，该函数返回之后，需要重新获取当前 CPU 的引用，不能再用调用该函数之前的 CPU 引用了，
    /// 因为当前任务可能已经被调度到别的 CPU 上。
    ///
    /// # Preconditions
    ///
    /// `current_task_inner` 必须是当前任务的锁，不能是别的任务的锁。
    ///
    /// # Panics
    ///
    /// 如果当前 CPU 上的自旋锁数量不为 1，则 panic。这意味着你只能对当前任务加锁，除此之外不能有别的锁。
    /// 否则多余的自旋锁会直到当前任务被调度回来才能被释放。
    ///
    /// # Notes
    ///
    /// 该函数假设当前任务一定会被调度回来，所以该函数一定会返回。
    ///
    /// 如果你想要回到调度循环，并且永远不会调度回来，考虑直接调用 [IrqArch::switch_context]。
    pub fn go_scheduler<'a>(mut current_task_inner: SpinMutexGuard<'a, TaskControlBlockInner>) {
        // SAFETY: 当前正在对 current_task_inner 加锁，所以中断是关闭的
        let cpu = unsafe { CPUManager::current_cpu() };
        if core::hint::unlikely(cpu.spinning_state.count != 1) {
            panic!("to_scheduler: spinning_state.count != 1");
        }
        // 这里需要保存当前 CPU 上的自旋锁的状态，并在调度回来之后恢复
        // 自旋锁的状态实际上并不是属于 CPU 的，而是属于当前任务的
        // 这里只需要保存一下施加第一个自旋锁之前的中断状态即可，因为当前自旋锁的适量必然是 1
        let interrupted = cpu.spinning_state.interrupted;
        let current_context = &mut current_task_inner.task_context as *mut _;
        // 这里会回到调度循环中，但是调度循环那边，在调度该任务的时候会持有一个锁
        // 这里的锁会在调度循环那里被释放掉
        unsafe { IrqArch::switch_context(current_context, &mut cpu.idle_task_context as *mut _) };
        // 这里需要重新获取当前 CPU 的引用，因为当前任务可能已经被调度到别的 CPU 上
        let cpu = unsafe { CPUManager::current_cpu() };
        cpu.spinning_state.interrupted = interrupted;
    }

    /// 当前正在运行的任务主动让出去
    ///
    /// # Panics
    ///
    /// 如果当前任务状态不为 RUNNING，则 panic
    // yield 在 rust 中是关键字，所以这里叫 yield_current
    pub fn yield_current() {
        let current_task = ProcessManager::current_task();
        let mut task_inner = current_task.lock();
        assert!(
            task_inner.status == TaskStatus::Running,
            "Current task status is not running: {:?}",
            task_inner.status
        );
        task_inner.status = TaskStatus::Ready;
        TaskScheduler::push(current_task.clone());
        ProcessManager::go_scheduler(task_inner);
    }

    /// 退出当前任务
    ///
    /// # Preconditions
    ///
    /// 调用前应确保应该 drop 掉的东西全都 drop 了，否则会出现资源泄露
    ///
    /// # Panics
    ///
    /// 如果当前持有其他的自旋锁，则 Panic
    pub fn exit() -> ! {
        let task = ProcessManager::current_task();
        let mut task_inner = task.lock();
        // SAFETY: 当前正在对 task_inner 加锁，所以中断是关闭的
        let cpu = unsafe { CPUManager::current_cpu() };
        if core::hint::unlikely(cpu.spinning_state.count != 1) {
            panic!("exit: still holding other locks");
        }
        let current_context = &mut task_inner.task_context as *mut _;
        // 由于不会回来了，所以这里需要把 task 这个 arc 给释放掉，防止这里一直占着个引用计数
        // 但是 task_inner 这个锁仍然需要保留，我们仍然需要确保调度过程中持有任务的锁
        // 所以我们有如下的操作：
        // SAFETY: 调度循环会把锁释放掉
        unsafe { task_inner.leak() };
        drop(task_inner);
        drop(task);
        // SAFETY: 此时我们还持有 task_inner 的锁，所以中断是关闭的
        unsafe { IrqArch::switch_context(current_context, &mut cpu.idle_task_context as *mut _) };
        unreachable!()
    }
}
