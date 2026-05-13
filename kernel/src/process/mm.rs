use crate::{
    arch::MMArch,
    mm::{
        KERNEL_SPACE, MemoryManagementArch,
        address::VirtAddr,
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission},
    },
    sync::spin::SpinMutex,
    utils::RecycleAllocator,
};
use lazy_static::lazy_static;

/// 内核栈大小，单位为页面
const KERNEL_STACK_SIZE: usize = 2;

pub struct KernelStack {
    id: usize,
}

impl KernelStack {
    /// 返回二元组的第一个元素为低地址，第二个元素为高地址
    pub fn range(&self) -> (VirtAddr, VirtAddr) {
        let all_stack_top =
            (1 << MMArch::VADDR_BITS_COUNT) - MMArch::TRAMPOLINE_PAGE_COUNT * MMArch::PAGE_SIZE;
        // 相邻两个内核栈之间还有一个空白页，便于栈溢出的时候及时发现
        let high = all_stack_top - self.id * (KERNEL_STACK_SIZE + 1) * MMArch::PAGE_SIZE;
        let low = high - KERNEL_STACK_SIZE * MMArch::PAGE_SIZE;
        (VirtAddr::new(low), VirtAddr::new(high))
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (base_vaddr, _) = self.range();
        KERNEL_SPACE.lock().remove(base_vaddr);
        KERNEL_STACK_ALLOCATOR.lock().dealloc(self.id);
    }
}

lazy_static! {
    static ref KERNEL_STACK_ALLOCATOR: SpinMutex<RecycleAllocator> =
        SpinMutex::new(RecycleAllocator::new(), "kernel_stack_allocator");
}

pub struct KernelStackAllocator;

impl KernelStackAllocator {
    pub fn alloc() -> KernelStack {
        let id = KERNEL_STACK_ALLOCATOR.lock().alloc();
        let res = KernelStack { id };
        let (base_vaddr, _) = res.range();
        let area = MemoryArea::new(
            base_vaddr,
            // 多分配一页
            MMArch::PAGE_SIZE * KERNEL_STACK_SIZE,
            MemoryPermission::Readable | MemoryPermission::Writable,
            MemoryAreaType::Private,
            "kernel_stack",
        );
        KERNEL_SPACE.lock().push(area);
        res
    }
}
