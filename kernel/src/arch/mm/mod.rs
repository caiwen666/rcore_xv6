mod init;
mod pte;

use crate::{
    arch::mm::pte::Sv39PTE,
    mm::{MemoryManagementArch, allocator::buddy::BuddyAllocator, mem_space::MemorySpace},
    sync::spin::SpinMutex,
};
use lazy_static::lazy_static;

pub struct RiscV64MMArch;

impl MemoryManagementArch for RiscV64MMArch {
    type PTE = Sv39PTE;

    /// 页面大小为 4096
    const PAGE_SIZE_SHIFT: usize = 12;
    /// 三级页表
    const PAGE_LEVELS: usize = 3;
    /// 每个页表的页表项数量为 512
    const PTE_COUNT_SHIFT: usize = 9;
    /// 跳板占了 1 个页面
    const TRAMPOLINE_PAGE_COUNT: usize = 1;
    /// 虚拟内存地址有 39 位，但是 SV39 要求第 39 位 (1-base) 为 1 的时候剩余高位必须都为 1
    /// 为了简单起见我们只用 38 位
    const VADDR_BITS_COUNT: usize = 38;

    fn init() {
        init::init();
    }

    fn activate(space: &MemorySpace) {
        use riscv::register::satp;
        let mut reg = satp::Satp::from_bits(0);
        reg.set_mode(satp::Mode::Sv39);
        // 当前实现直接把页表全刷了，所以 asid 无所谓
        reg.set_asid(0);
        reg.set_ppn(unsafe { space.table().paddr().inner() >> 12 });
        unsafe {
            riscv::register::satp::write(reg);
            core::arch::asm!("sfence.vma");
        }
    }
}

lazy_static! {
    /// 全局的页帧分配器
    pub static ref FRAME_ALLOCATOR: SpinMutex<BuddyAllocator> =
        SpinMutex::new(BuddyAllocator::new(), "frame_allocator");
}
