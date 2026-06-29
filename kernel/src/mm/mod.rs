pub mod address;
pub mod allocator;
pub mod mem_space;
pub mod page_table;
mod syscall;

use crate::{
    mm::{address::PhysAddr, mem_space::MemorySpace, page_table::PageTableEntry},
    sync::spin::SpinMutex,
};
use lazy_static::lazy_static;

pub trait MemoryManagementArch {
    type PTE: PageTableEntry;

    /// 页面大小的 shift
    ///
    /// 例如 PAGE_SIZE_SHIFT = 12 表示页面大小为 4096
    const PAGE_SIZE_SHIFT: usize;

    /// 页表层级数
    const PAGE_LEVELS: usize;
    /// 每个页表的页表项数量 shift
    ///
    /// 例如 PAGE_ENTRY_COUNT_SHIFT = 9 表示每个页表有 512 个页表项
    const PTE_COUNT_SHIFT: usize;

    /// 跳板占了多少个页面
    const TRAMPOLINE_PAGE_COUNT: usize;

    /// 虚拟地址有多少位
    const VADDR_BITS_COUNT: usize;

    /// 页面大小
    const PAGE_SIZE: usize = 1 << Self::PAGE_SIZE_SHIFT;
    /// 页表项大小
    const PTE_SIZE: usize = core::mem::size_of::<Self::PTE>();
    /// 页表项数量
    const PTE_COUNT: usize = 1 << Self::PTE_COUNT_SHIFT;
    /// 页表大小
    const PAGE_TABLE_SIZE: usize = Self::PTE_COUNT * Self::PTE_SIZE;

    /// 初始化内存
    fn init();

    /// 进入某个内存空间
    fn activate(space: &MemorySpace);

    /// 刷新当前 CPU 的 TLB
    fn local_flush_tlb();

    /// 请求其他 CPU 刷新 TLB
    ///
    /// # Safety
    ///
    /// 调用时需要关闭中断
    unsafe fn tlb_shootdown();
}

/// 物理内存区域
#[derive(Clone, Copy, Debug)]
pub struct PhysMemoryArea {
    /// 名称
    pub name: &'static str,
    /// 物理基地址
    pub base: PhysAddr,
    /// 物理内存大小
    pub size: usize,
    /// 类型
    pub kind: PhysMemoryAreaKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysMemoryAreaKind {
    /// 设备空间
    Device,
    /// 主内存
    MainMemory,
}

lazy_static! {
    pub static ref KERNEL_SPACE: SpinMutex<MemorySpace> =
        SpinMutex::new(MemorySpace::create(), "kernel_space");
}
