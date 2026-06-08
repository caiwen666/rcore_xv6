pub mod buddy;
pub mod bump;
pub mod kernel;
pub mod slab;

use crate::{
    arch::MMArch,
    mm::{MemoryManagementArch, address::PhysAddr, allocator::buddy::BuddyAllocator},
    sync::spin::SpinMutex,
};
use core::fmt::Debug;
use lazy_static::lazy_static;

/// 页帧分配器
pub trait PageFrameAllocator {
    /// 分配指定数量的页帧
    ///
    /// 分配出来的页帧中存放的数据不保证清零
    ///
    /// # Returns
    ///
    /// - 返回页帧的起始地址
    /// - 如果分配失败，则返回 None
    fn alloc(&mut self, count: usize) -> Option<PhysAddr>;
    /// 释放某一地址开始的指定数量的页帧
    ///
    /// # Preconditions
    ///
    /// `addr` 和 `count` 需要与调用 `alloc` 时传入的参数和返回值一致，
    /// 否则会导致未定义行为
    fn dealloc(&mut self, addr: PhysAddr, count: usize);
}

/// 页帧
pub struct PageFrame {
    addr: PhysAddr,
    count: usize,
}

impl PageFrame {
    pub fn new(addr: PhysAddr, count: usize) -> Self {
        Self { addr, count }
    }

    pub fn addr(&self) -> PhysAddr {
        self.addr
    }

    #[expect(unused)]
    pub fn count(&self) -> usize {
        self.count
    }

    /// 将页的全部内容清零
    pub fn clear(&mut self) {
        unsafe {
            core::ptr::write_bytes(self.addr.get_mut::<u8>(), 0, MMArch::PAGE_SIZE * self.count)
        };
    }
}

impl Drop for PageFrame {
    /// PageFrame 必然是通过全局页帧分配器得来的，因此调用全局页帧分配器释放页帧
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().dealloc(self.addr, self.count);
    }
}

impl Debug for PageFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "[{:#X}, {:#X})({} pages)",
            self.addr.inner(),
            self.addr.inner() + self.count * MMArch::PAGE_SIZE,
            self.count
        )
    }
}

lazy_static! {
    /// 全局的页帧分配器
    pub static ref FRAME_ALLOCATOR: SpinMutex<BuddyAllocator> =
        SpinMutex::new(BuddyAllocator::new(), "frame_allocator");
}

/// 使用全局页帧分配器分配一个页帧
pub fn alloc_frame(count: usize) -> Option<PageFrame> {
    let paddr = FRAME_ALLOCATOR.lock().alloc(count)?;
    Some(PageFrame::new(paddr, count))
}
