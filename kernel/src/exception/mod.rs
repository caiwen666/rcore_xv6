pub trait InterruptArch {
    /// 开启中断
    fn enable_interrupt();
    /// 关闭中断
    fn disable_interrupt();
    /// 获取当前中断状态，为 true 表示中断开启，为 false 表示中断关闭
    fn get_interrupt_state() -> bool;
}
