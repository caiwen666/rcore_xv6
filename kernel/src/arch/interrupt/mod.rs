use crate::exception::InterruptArch;
use riscv::register::sstatus;

pub struct RiscV64InterruptArch;

impl InterruptArch for RiscV64InterruptArch {
    #[inline]
    fn enable_interrupt() {
        let mut status = sstatus::read();
        status.set_sie(true);
        unsafe { sstatus::write(status) };
    }

    #[inline]
    fn disable_interrupt() {
        let mut status = sstatus::read();
        status.set_sie(false);
        unsafe { sstatus::write(status) };
    }

    #[inline]
    fn get_interrupt_state() -> bool {
        let status = sstatus::read();
        status.sie()
    }
}

#[expect(unused)]
pub fn init_timer() {
    // TODO
}
