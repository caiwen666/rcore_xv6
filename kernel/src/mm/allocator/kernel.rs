use core::alloc::{GlobalAlloc, Layout};

use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::PhysAddr,
        allocator::{FRAME_ALLOCATOR, PageFrameAllocator, slab::SlabAllocator},
    },
    sync::spin::SpinMutex,
};
use lazy_static::lazy_static;

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

lazy_static! {
    static ref SLAB_ALLOCATOR: SpinMutex<SlabAllocator> =
        SpinMutex::new(SlabAllocator::new(), "slab_allocator");
}

#[global_allocator]
pub static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;

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
