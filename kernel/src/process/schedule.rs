use crate::{
    arch::IrqArch,
    exception::InterruptArch,
    process::{
        cpu::CPUManager,
        task::{TaskControlBlock, TaskStatus},
    },
    sync::spin::SpinMutex,
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
            cpu.current_task = Some(task.clone());
            let mut task_inner = task.lock();
            task_inner.status = TaskStatus::Running;
            // 不用释放 task_inner
            // 如果这里的切换回到了 CPU::go_scheduler，
            // 其要求必须是拿住线程的锁的，所以 CPU::go_scheduler 的调用者会释放
            // 如果这里的切换去到了 task_entry，这个函数最开始也会把锁释放掉
            IrqArch::switch_context(
                &mut cpu.idle_task_context as *mut _,
                &mut task_inner.context as *mut _,
            );
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
        IrqArch::enable_interrupt();
        kthread_entry();
    }
    unimplemented!()
}
