use crate::process::{ProcessManager, cpu::CPUManager, sleep::check_sleep_timer};
use core::sync::atomic::{AtomicUsize, Ordering};

/// 时钟中断的间隔，单位为毫秒
pub const TIMER_INTERVAL: usize = 1;

// 在 64 位架构上，usize 足以保证系统运行数万年也不会出现 jiffies 溢出
pub static JIFFIES: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn time_us() -> usize {
    jiffies() * TIMER_INTERVAL * 1000
}

/// 时钟中断处理
///
/// # Parameters
///
/// - `from_kernel`: 是否在内核态触发的中断
///
/// # Safety
///
/// 调用时需要保证中断关闭
pub unsafe fn timer_handler(from_kernel: bool) {
    // SAFETY: 当前函数的调用者已经保证了关闭了中断
    let cpu = unsafe { CPUManager::current_cpu() };
    if cpu.id == 0 {
        JIFFIES.fetch_add(1, Ordering::Relaxed);
        check_sleep_timer();
    }
    // 如果从用户态过来的，当前 CPU 上一定是有任务的
    // 如果从内核态过来的，当前 CPU 上未必有任务，需要再判断一下
    if !from_kernel || (from_kernel && cpu.current_task.is_some()) {
        ProcessManager::yield_current();
    }
}

#[inline]
pub fn jiffies() -> usize {
    JIFFIES.load(Ordering::Relaxed)
}
