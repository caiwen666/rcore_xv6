use crate::{
    arch::MMArch,
    mm::{
        KERNEL_SPACE, MemoryManagementArch,
        address::VirtAddr,
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission, MemorySpace},
    },
    process::context::TRAP_CONTEXT_PAGE_COUNT,
    sync::spin::SpinMutex,
    utils::RecycleAllocator,
};
use lazy_static::lazy_static;
use xmas_elf::{ElfFile, program::Type};

/// 内核栈大小，单位为页面
const KERNEL_STACK_SIZE: usize = 4;
/// 用户栈大小，单位为页面
const USER_STACK_SIZE: usize = 4;

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
        let mut allocator = KERNEL_STACK_ALLOCATOR.lock();
        allocator.dealloc(self.id);
        // 内存释放完毕才删除内存区域
        KERNEL_SPACE.lock().remove(base_vaddr);
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
            MMArch::PAGE_SIZE * KERNEL_STACK_SIZE,
            MemoryPermission::Readable | MemoryPermission::Writable,
            MemoryAreaType::Private,
            "kernel_stack",
        );
        KERNEL_SPACE.lock().push(area);
        res
    }
}

/// 根据 tid 计算线程用户栈地址的 trap 上下文地址
///
/// # Returns
///
/// 返回二元组为 (l, r)，表示在用户栈在 [l, r) 这个地址区间上
pub fn ustack_vaddr(tid: usize) -> (VirtAddr, VirtAddr) {
    let all_top =
        (1 << MMArch::VADDR_BITS_COUNT) - MMArch::TRAMPOLINE_PAGE_COUNT * MMArch::PAGE_SIZE;
    // 从 all_top 开始，每个进程都占了三个部分，从高地址往低地址依次是：
    // trap 上下文 -> 用户栈 -> 空白页
    let part_high =
        all_top - tid * (TRAP_CONTEXT_PAGE_COUNT + USER_STACK_SIZE + 1) * MMArch::PAGE_SIZE;
    let high = part_high - TRAP_CONTEXT_PAGE_COUNT * MMArch::PAGE_SIZE;
    let low = high - USER_STACK_SIZE * MMArch::PAGE_SIZE;
    (VirtAddr::new(low), VirtAddr::new(high))
}

/// 根据 tid 计算线程的 trap 上下文地址
///
/// # Returns
///
/// 返回 trap 上下文的起始地址
pub fn trap_context_vaddr(tid: usize) -> VirtAddr {
    let (_, high) = ustack_vaddr(tid);
    high
}

impl MemorySpace {
    pub(super) fn from_elf(elf: &ElfFile) -> Self {
        let mut memory_space = MemorySpace::create();
        elf.program_iter()
            .filter(|ph| ph.get_type().unwrap() == Type::Load)
            .for_each(|ph| {
                let mut permission = MemoryPermission::empty();
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    permission |= MemoryPermission::Readable;
                }
                if ph_flags.is_write() {
                    permission |= MemoryPermission::Writable;
                }
                if ph_flags.is_execute() {
                    permission |= MemoryPermission::Executable;
                }
                permission |= MemoryPermission::UserAccessible;
                let vaddr = ph.virtual_addr() as usize / MMArch::PAGE_SIZE * MMArch::PAGE_SIZE;
                let offset = ph.virtual_addr() as usize % MMArch::PAGE_SIZE;
                let size = offset + ph.mem_size() as usize;
                let mut area = MemoryArea::new(
                    VirtAddr::new(vaddr),
                    size,
                    permission,
                    MemoryAreaType::Private,
                    "elf",
                );
                area.write_data(
                    offset,
                    &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize],
                );
                memory_space.push(area);
            });
        memory_space
    }
}
