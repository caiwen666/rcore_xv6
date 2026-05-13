use crate::{arch::IrqArch, exception::InterruptArch, mm::address::VirtAddr};

pub trait TaskContext: Clone {
    /// 生成一个空的上下文
    fn zero_init() -> Self;
    /// 生成一个将要回到 task_entry，且 sp 指向内核栈栈顶的上下文
    fn new(kernel_stack: VirtAddr) -> Self;
}

pub type ArchTaskContext = <IrqArch as InterruptArch>::TaskContext;
