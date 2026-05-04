#[inline]
pub unsafe fn write_tp(val: usize) {
    unsafe { core::arch::asm!("mv tp, {0}", in(reg) val) };
}

#[inline]
pub fn read_tp() -> usize {
    let val: usize;
    unsafe { core::arch::asm!("mv {0}, tp", out(reg) val) };
    val
}
