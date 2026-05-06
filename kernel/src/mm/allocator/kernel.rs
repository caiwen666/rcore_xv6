use core::alloc::{GlobalAlloc, Layout};

use crate::{
    arch::{FRAME_ALLOCATOR, MMArch},
    mm::{
        MemoryManagementArch,
        address::PhysAddr,
        allocator::{PageFrameAllocator, slab::SLAB_ALLOCATOR},
    },
};

pub struct KernelAllocator;

/// 判断选择 buddy 分配还是 slab 分配
///
/// 返回 true 则说明应该选择 buddy
fn select_allocator(layout: Layout) -> bool {
    // 大于 4kb 时一定要使用 buddy 分配，因为 slab 分配器会调用 GlobalAlloc
    // 来扩容自己管理的内存
    layout.size() > 2048
}

impl KernelAllocator {
    pub fn alloc_in_buddy(&self, layout: Layout) -> *mut u8 {
        let count = layout.size().div_ceil(MMArch::PAGE_SIZE);
        FRAME_ALLOCATOR
            .lock()
            .alloc(count)
            .expect("Failed to allocate kernel memory in buddy")
            .get_mut::<u8>()
    }

    pub fn dealloc_in_buddy(&self, ptr: *mut u8, layout: Layout) {
        let count = layout.size().div_ceil(MMArch::PAGE_SIZE);
        FRAME_ALLOCATOR
            .lock()
            .dealloc(PhysAddr::new(ptr as usize), count);
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if select_allocator(layout) {
            self.alloc_in_buddy(layout)
        } else {
            SLAB_ALLOCATOR.lock().alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if select_allocator(layout) {
            self.dealloc_in_buddy(ptr, layout);
        } else {
            SLAB_ALLOCATOR.lock().dealloc(ptr, layout);
        }
    }
}
