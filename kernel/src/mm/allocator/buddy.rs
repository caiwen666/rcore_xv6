use crate::{
    arch::MMArch,
    mm::{MemoryManagementArch, address::PhysAddr, allocator::PageFrameAllocator},
};
use buddy_system_allocator::Heap;
use core::{alloc::Layout, ptr::NonNull};

pub struct BuddyAllocator {
    inner: Heap<32>,
}

impl BuddyAllocator {
    pub fn new() -> Self {
        Self { inner: Heap::new() }
    }

    /// 将指定地址范围内的内存区域添加到分配器中管理
    pub fn add_area(&mut self, start: PhysAddr, end: PhysAddr) {
        unsafe { self.inner.add_to_heap(start.inner(), end.inner()) };
    }
}

impl PageFrameAllocator for BuddyAllocator {
    fn alloc(&mut self, count: usize) -> Option<PhysAddr> {
        let layout = unsafe {
            Layout::from_size_align_unchecked(count * MMArch::PAGE_SIZE, MMArch::PAGE_SIZE)
        };
        let addr = self.inner.alloc(layout).ok()?;
        Some(PhysAddr::new(addr.addr().get()))
    }

    fn dealloc(&mut self, addr: PhysAddr, count: usize) {
        let layout = unsafe {
            Layout::from_size_align_unchecked(count * MMArch::PAGE_SIZE, MMArch::PAGE_SIZE)
        };
        unsafe {
            self.inner.dealloc(
                NonNull::new(addr.get_mut::<u8>()).expect("addr is Illegal"),
                layout,
            )
        };
    }
}
