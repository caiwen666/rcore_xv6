pub mod cpu;
mod entry;
mod fdt;
pub mod interrupt;
mod mm;
mod register;

pub use interrupt::RiscV64InterruptArch as IrqArch;
pub use mm::RiscV64MMArch as MMArch;
