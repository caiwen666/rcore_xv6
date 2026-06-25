/// 获取当前 CPU 的 id
///
/// # Safety
///
/// 调用时需要保证中断关闭
pub unsafe fn cpu_id() -> usize {
    super::register::tp::read_tp()
}
