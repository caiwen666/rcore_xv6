use crate::{
    arch::MMArch,
    mm::{MemoryManagementArch, address::PhysAddr, allocator::PageFrameAllocator},
};

pub struct BumpAllocator {
    offset: usize,
}

impl BumpAllocator {
    /// # Parameters
    ///
    /// - `offset` - 分配器的起始偏移量
    #[expect(unused, reason = "当前内核启动过程比较简单，不需要 BumpAllocator")]
    pub fn new(offset: usize) -> Self {
        Self { offset }
    }

    ///获取当前分配器的偏移量
    #[expect(unused)]
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl PageFrameAllocator for BumpAllocator {
    fn alloc(&mut self, count: usize) -> Option<PhysAddr> {
        let paddr = PhysAddr::new(self.offset);
        self.offset += count * MMArch::PAGE_SIZE;
        Some(paddr)
    }

    fn dealloc(&mut self, _addr: PhysAddr, _count: usize) {
        unimplemented!()
    }
}
