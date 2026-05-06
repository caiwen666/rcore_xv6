pub mod cpu;
mod entry;
pub mod interrupt;
mod mm;
mod register;

pub use interrupt::RiscV64InterruptArch as IrqArch;
pub use mm::{FRAME_ALLOCATOR, RiscV64MMArch as MMArch};
