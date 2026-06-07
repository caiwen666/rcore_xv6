use crate::process::{cpu::CPUManager, timer::check_sleep_timer};
use core::sync::atomic::{AtomicUsize, Ordering};

/// 时钟中断的间隔，单位为毫秒
pub const TIMER_INTERVAL: usize = 1;

// 在 64 位架构上，usize 足以保证系统运行数万年也不会出现 jiffies 溢出
pub static JIFFIES: AtomicUsize = AtomicUsize::new(0);

/// 时钟中断处理
///
/// # Parameters
///
/// - `from_kernel`: 是否在内核态触发的中断
///
/// # Safety
///
/// 调用时需要保证中断关闭
pub unsafe fn timer_handler(_from_kernel: bool) {
    // SAFETY: 当前函数的调用者已经保证了关闭了中断
    let cpu = unsafe { CPUManager::current_cpu() };
    if cpu.id == 0 {
        JIFFIES.fetch_add(1, Ordering::Relaxed);
        check_sleep_timer();
    }
    cpu.yield_current_task();
}

#[inline]
pub fn jiffies() -> usize {
    JIFFIES.load(Ordering::Relaxed)
}
