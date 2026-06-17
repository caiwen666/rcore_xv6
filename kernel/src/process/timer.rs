use core::{cmp::Ordering, time::Duration};

use crate::{
    exception::timer::{TIMER_INTERVAL, jiffies},
    process::{
        cpu::CPUManager,
        schedule::TaskScheduler,
        task::{TaskControlBlock, TaskStatus},
    },
    sync::spin::SpinMutex,
};
use alloc::{collections::binary_heap::BinaryHeap, sync::Arc};
use lazy_static::lazy_static;

#[inline]
pub fn time_us() -> usize {
    jiffies() * TIMER_INTERVAL * 1000
}

struct SleepTimer {
    expire_us: usize,
    task: Arc<TaskControlBlock>,
}

impl PartialEq for SleepTimer {
    fn eq(&self, other: &Self) -> bool {
        self.expire_us == other.expire_us
    }
}
impl Eq for SleepTimer {}
impl PartialOrd for SleepTimer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SleepTimer {
    fn cmp(&self, other: &Self) -> Ordering {
        other.expire_us.cmp(&self.expire_us)
    }
}

lazy_static! {
    static ref SLEEP_TIMER_QUEUE: SpinMutex<BinaryHeap<SleepTimer>> =
        SpinMutex::new(BinaryHeap::new(), "sleep_timer");
}

/// 休眠当前线程，直到 `expire_us` 时间后唤醒
///
/// # Parameters
///
/// - `expire_us`: 到期时间，单位为微秒
///
/// # Preconditions
///
/// 调用本函数时，不能持有自旋锁
pub fn sleep_with_expire(expire_us: usize) {
    // 没用 condvar 来做睡眠和唤醒
    // 用 condvar 的话需要取 condvar 加入堆之后的引用来睡眠
    // 而 BinaryHeap 不支持插入元素后返回元素在堆中引用
    let current_task = CPUManager::current_task().unwrap();
    let sleep_timer = SleepTimer {
        expire_us,
        task: current_task.clone(),
    };
    let mut queue = SLEEP_TIMER_QUEUE.lock();
    queue.push(sleep_timer);
    let mut task_inner = current_task.lock();
    // 拿到 task_inner 之后再释放 queue，防止我们还没完成睡眠就被唤醒了
    drop(queue);
    task_inner.status = TaskStatus::Blocked;
    let current_context = &mut task_inner.task_context as *mut _;
    // 有可能回到调度循环之后，当前任务被杀死，然后永远回不来了
    // 所以这里需要搞一个类似 [CPU::yield_current_task] 的操作
    unsafe { task_inner.leak() };
    drop(task_inner);
    drop(current_task);
    let cpu = unsafe { CPUManager::current_cpu() };
    cpu.go_scheduler(current_context);

    let cpu = unsafe { CPUManager::current_cpu() };
    let current_task = cpu.current_task.clone().unwrap();
    // SAFETY: 到这里说明从调度循环回来了。在回来之前，调度循环会加锁
    unsafe { current_task.unlock() };
}

/// 休眠当前线程，直到过了 `interval` 之后唤醒
///
/// # Panics
///
/// - 如果间隔时间转为微秒后超过 usize::MAX，则 panic
/// - 如果当前系统的计时器时间加上间隔时间超过 usize::MAX，则 panic
///
/// # Preconditions
///
/// 调用本函数时，不能持有自旋锁
#[inline]
pub fn sleep_with_interval(interval: Duration) {
    assert!(
        interval.as_micros() <= usize::MAX as u128,
        "interval is too long"
    );
    let current_time = time_us();
    sleep_with_expire(
        current_time
            .checked_add(interval.as_micros() as usize)
            .expect("interval is too long"),
    );
}

pub fn check_sleep_timer() {
    let mut queue = SLEEP_TIMER_QUEUE.lock();
    let current_time = time_us();
    while let Some(sleep_timer) = queue.peek() {
        if sleep_timer.expire_us <= current_time {
            let mut task_inner = sleep_timer.task.lock();
            task_inner.status = TaskStatus::Ready;
            TaskScheduler::push(sleep_timer.task.clone());
            drop(task_inner);
            queue.pop();
        } else {
            break;
        }
    }
}
