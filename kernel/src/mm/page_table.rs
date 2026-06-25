use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::{PhysAddr, VirtAddr},
        mem_space::MemoryPermission,
    },
};

/// 页表项
pub trait PageTableEntry: Clone + Copy + 'static {
    /// 叶子节点，创建后得到的页表项必须满足是有效的
    fn new_leaf(paddr: PhysAddr, permission: MemoryPermission) -> Self;
    /// 非叶子节点，指向下一级的页表物理地址
    fn new_non_leaf(paddr: PhysAddr) -> Self;
    /// 空白页表项
    fn empty() -> Self;
    /// 页表项是否有效
    fn is_valid(&self) -> bool;
    /// 获取页表项指向的物理地址
    fn paddr(&self) -> PhysAddr;
    /// 获取页表项的权限
    fn permission(&self) -> MemoryPermission;
}

#[derive(Debug)]
pub struct PageTable {
    /// 当前页表示的虚拟空间的起始地址
    base: VirtAddr,
    /// 当前页表所在的物理地址
    paddr: PhysAddr,
    /// 当前页表的层级，从 0 开始，0 级是最小的
    level: usize,
}

type PTE = <MMArch as MemoryManagementArch>::PTE;

impl PageTable {
    pub fn new(base: VirtAddr, paddr: PhysAddr, level: usize) -> Self {
        Self { base, paddr, level }
    }

    pub fn level(&self) -> usize {
        self.level
    }

    pub fn paddr(&self) -> PhysAddr {
        self.paddr
    }

    /// 获取第 i 个页表项表示的虚拟空间的起始地址
    ///
    /// # Panics
    ///
    /// 如果 i 超出了页表项的索引范围，则 panic
    pub fn entry_base(&self, i: usize) -> VirtAddr {
        if core::hint::unlikely(i >= MMArch::PTE_COUNT) {
            panic!("Index out of range: i = {}", i);
        }
        let shift = self.level * MMArch::PTE_COUNT_SHIFT + MMArch::PAGE_SIZE_SHIFT;
        self.base + (i << shift)
    }

    /// 获取第 i 个页表项的物理地址
    ///
    /// # Panics
    ///
    /// 如果 i 超出了页表项的索引范围，则 panic
    pub fn entry_paddr(&self, i: usize) -> PhysAddr {
        if core::hint::unlikely(i >= MMArch::PTE_COUNT) {
            panic!("Index out of range: i = {}", i);
        }
        self.paddr + (i * MMArch::PTE_SIZE)
    }

    /// 获取第 i 个页表项的可变引用
    pub fn get_mut(&mut self, i: usize) -> &mut PTE {
        if core::hint::unlikely(i >= MMArch::PTE_COUNT) {
            panic!("Index out of range: i = {}", i);
        }
        self.entry_paddr(i).get_mut::<PTE>()
    }

    /// 获取第 i 个页表项的不可变引用
    pub fn get(&self, i: usize) -> &PTE {
        if core::hint::unlikely(i >= MMArch::PTE_COUNT) {
            panic!("Index out of range: i = {}", i);
        }
        self.entry_paddr(i).get::<PTE>()
    }

    /// 设置第 i 个页表项
    pub fn set(&mut self, i: usize, pte: PTE) {
        let ptr = self.get_mut(i);
        *ptr = pte;
    }

    /// 获取指定虚拟地址在当前页表中的索引
    ///
    /// # Returns
    ///
    /// 如果虚拟地址不在当前页表的范围内，则返回 None
    pub fn index_of(&self, va: VirtAddr) -> Option<usize> {
        let shift = MMArch::PTE_COUNT_SHIFT * self.level + MMArch::PAGE_SIZE_SHIFT;
        // 当前层级下，一个页表项覆盖的内存大小
        let pte_memory_size = 1 << shift;
        // 整张页表覆盖的范围
        let table_span = pte_memory_size * MMArch::PTE_COUNT;
        if core::hint::unlikely(va < self.base || va >= self.base + table_span) {
            None
        } else {
            Some((va.inner() - self.base.inner()) >> shift)
        }
    }

    /// 获取第 i 个页表项指向的下一级页表
    ///
    /// # Returns
    ///
    /// 返回 `None` 则表示下一级页表不存在
    ///
    /// # Panics
    ///
    /// 如果当前页表的层级为 0，则 panic
    ///
    /// # Safety
    ///
    /// - 需要确保引用 PageTable 的生命周期不超过 MemorySpace 的生命周期
    /// - 你应该把返回值视为一个对 MemorySpace 的可变引用，遵循可变引用的所有规则
    pub unsafe fn next_level_table(&self, i: usize) -> Option<PageTable> {
        if core::hint::unlikely(self.level == 0) {
            panic!("Cannot get next level table of root page table");
        }
        let pte = self.get(i);
        if !pte.is_valid() {
            return None;
        }
        let next_level_table_paddr = pte.paddr();
        Some(PageTable::new(
            self.entry_base(i),
            next_level_table_paddr,
            self.level - 1,
        ))
    }
}
