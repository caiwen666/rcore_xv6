/// 获取当前 CPU 的 id
pub fn cpu_id() -> usize {
    super::register::tp::read_tp()
}
