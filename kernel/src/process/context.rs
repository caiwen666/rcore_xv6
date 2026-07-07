use crate::{
    arch::{IrqArch, MMArch},
    exception::InterruptArch,
    mm::{MemoryManagementArch, address::VirtAddr},
};

pub trait TaskContext: Clone {
    /// 生成一个空的上下文
    fn zero_init() -> Self;
    /// 生成一个将要回到 task_entry，且 sp 指向内核栈栈顶的上下文
    fn new(kernel_stack: VirtAddr) -> Self;
}

pub type ArchTaskContext = <IrqArch as InterruptArch>::TaskContext;

pub trait TrapContext: Clone {
    /// 生成一个 trap 上下文
    ///
    /// # Parameters
    ///
    /// - `kstack`: 任务的内核栈的地址
    fn new(kstack: VirtAddr) -> Self;
    /// 设置回到用户态之后执行指令的地址
    fn set_pc(self, pc: VirtAddr) -> Self;
    /// 获取用户态陷入内核态时执行指令的地址
    #[expect(unused)]
    fn pc(&self) -> VirtAddr;
    /// 设置用户栈地址
    fn set_ustack(self, ustack: VirtAddr) -> Self;
    /// 设置 tls 基址
    fn set_tls_base(self, tls_base: VirtAddr) -> Self;
    /// 设置返回值
    fn set_return_value(&mut self, return_value: usize);
}

pub type ArchTrapContext = <IrqArch as InterruptArch>::TrapContext;

pub const TRAP_CONTEXT_PAGE_COUNT: usize =
    core::mem::size_of::<ArchTaskContext>().div_ceil(MMArch::PAGE_SIZE);
// 目前暂不支持 trap 上下文大于一页（应该也没有架构是需要这样的）
const _: () = assert!(
    TRAP_CONTEXT_PAGE_COUNT == 1,
    "Unsupported trap context page count"
);
