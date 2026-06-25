pub mod syscall;
pub mod timer;

pub trait InterruptArch {
    type TaskContext: crate::process::context::TaskContext;
    type TrapContext: crate::process::context::TrapContext;

    /// 中断初始化
    fn init();
    /// 开启中断
    fn enable_interrupt();
    /// 关闭中断
    fn disable_interrupt();
    /// 获取当前中断状态，为 true 表示中断开启，为 false 表示中断关闭
    fn get_interrupt_state() -> bool;
    /// 切换上下文
    ///
    /// # Safety
    ///
    /// 调用者需要确保调用时的中断是关闭的
    unsafe fn switch_context(
        current_context: *mut Self::TaskContext,
        next_context: *mut Self::TaskContext,
    );
    /// 回到用户态，调用后不再返回
    ///
    /// # Panics
    ///
    /// 调用时不能持有任何的自旋锁
    ///
    /// # Preconditions
    ///
    /// 调用前请确保所有该 drop 的资源都被 drop 掉了，否则会导致资源泄露
    fn return_to_user() -> !;
}
