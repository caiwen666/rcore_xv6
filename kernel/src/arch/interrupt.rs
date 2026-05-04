use riscv::register::sstatus;

/// 关闭中断
#[inline]
pub fn disable_interrupt() {
    let mut status = sstatus::read();
    status.set_sie(false);
    unsafe { sstatus::write(status) };
}

/// 开启中断
#[inline]
pub fn enable_interrupt() {
    let mut status = sstatus::read();
    status.set_sie(true);
    unsafe { sstatus::write(status) };
}

/// 获取当前中断状态
#[inline]
pub fn get_interrupt_state() -> bool {
    let status = sstatus::read();
    status.sie()
}

pub fn init_timer() {
    // TODO
}
