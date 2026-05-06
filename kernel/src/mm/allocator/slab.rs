use core::{alloc::Layout, ptr::NonNull};

use crate::{KERNEL_ALLOCATOR, sync::spin::SpinMutex};
use alloc::boxed::Box;
use lazy_static::lazy_static;
use slabmalloc::*;

lazy_static! {
    pub static ref SLAB_ALLOCATOR: SpinMutex<SlabAllocator> =
        SpinMutex::new(SlabAllocator::new(), "slab_allocator");
}

pub struct SlabAllocator {
    inner: ZoneAllocator<'static>,
}

impl SlabAllocator {
    pub fn new() -> Self {
        Self {
            inner: ZoneAllocator::new(),
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        match self.inner.allocate(layout) {
            Ok(ptr) => ptr.as_ptr(),
            Err(AllocationError::OutOfMemory) => {
                // 这里就自动调用 GlobalAlloc 分配了
                let boxed_page = ObjectPage::new();
                let leaked_page = Box::leak(boxed_page);
                unsafe {
                    self.inner
                        .refill(layout, leaked_page)
                        .expect("Failed to refill slab allocator")
                };
                self.inner
                    .allocate(layout)
                    .expect("Failed to allocate memory in slab allocator")
                    .as_ptr()
            }
            Err(AllocationError::InvalidLayout) => {
                panic!("Invalid layout: {:?}", layout);
            }
        }
    }

    pub fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let nptr = NonNull::new(ptr).expect("Invalid pointer");
        unsafe {
            self.inner
                .deallocate(nptr, layout, &SlabCallback)
                .expect("Failed to deallocate kernel memory")
        };
    }
}

/// Slab 归还内存给 Buddy 的回调
struct SlabCallback;
impl CallBack for SlabCallback {
    unsafe fn free_slab_page(&self, ptr: *mut u8, size: usize) {
        KERNEL_ALLOCATOR
            .dealloc_in_buddy(ptr, unsafe { Layout::from_size_align_unchecked(size, 1) });
    }
}
