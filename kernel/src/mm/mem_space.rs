use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::{PhysAddr, VirtAddr},
        allocator::{PageFrame, alloc_frame},
        page_table::{PageTable, PageTableEntry},
    },
    println,
    utils::BlockIterator,
};
use alloc::collections::btree_map::BTreeMap;
use bitflags::bitflags;
use core::{fmt::Debug, ops::Bound};

bitflags! {
    #[derive(Clone, Copy, Eq, PartialEq)]
    pub struct MemoryPermission: u8 {
        /// 是否可读
        const Readable = 1 << 1;
        /// 是否可写
        const Writable = 1 << 2;
        /// 是否可执行
        const Executable = 1 << 3;
        /// 用户态是否可访问
        const UserAccessible = 1 << 4;
    }
}

impl Debug for MemoryPermission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.contains(MemoryPermission::Readable) {
            write!(f, "R")?;
        }
        if self.contains(MemoryPermission::Writable) {
            write!(f, "W")?;
        }
        if self.contains(MemoryPermission::Executable) {
            write!(f, "X")?;
        }
        if self.contains(MemoryPermission::UserAccessible) {
            write!(f, "U")?;
        }
        Ok(())
    }
}

/// 内存区域类型
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryAreaType {
    /// 私有内存区域
    Private,
    /// 恒等映射。一般用于建立内核空间，不分配物理页，直接在页表中映射到物理地址
    Identical,
}

/// 内存区域
#[derive(Debug)]
pub struct MemoryArea {
    /// 名称
    name: &'static str,
    /// 起始虚拟地址
    base_vaddr: VirtAddr,
    /// 大小
    size: usize,
    /// 类型
    area_type: MemoryAreaType,
    /// 权限位
    permission: MemoryPermission,
    /// 私有的物理页
    ///
    /// 需要时刻保持 BTreeMap 里的物理页数量和 size 对应的物理页数量一致，
    /// 并且这里面的物理页也是按物理地址从小到大的顺序和虚拟页映射的
    private_frames: BTreeMap<PhysAddr, PageFrame>,
}

impl MemoryArea {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn size(&self) -> usize {
        self.size
    }
    pub fn base_vaddr(&self) -> VirtAddr {
        self.base_vaddr
    }
    pub fn area_type(&self) -> MemoryAreaType {
        self.area_type
    }
    pub fn permission(&self) -> MemoryPermission {
        self.permission
    }

    pub fn private_frame(&self) -> &BTreeMap<PhysAddr, PageFrame> {
        &self.private_frames
    }

    /// 创建内存区域
    ///
    /// 实际大小会向上对齐到整页大小
    ///
    /// # Panics
    ///
    /// `base_vaddr` 必须是 [MMArch::PAGE_SIZE] 的倍数，否则会 panic
    pub fn new(
        base_vaddr: VirtAddr,
        size: usize,
        permission: MemoryPermission,
        area_type: MemoryAreaType,
        name: &'static str,
    ) -> MemoryArea {
        assert!(
            base_vaddr.inner().is_multiple_of(MMArch::PAGE_SIZE),
            "MemoryArea::new: base vaddr is not page aligned {:?}",
            base_vaddr
        );
        let count = size.div_ceil(MMArch::PAGE_SIZE);
        match area_type {
            MemoryAreaType::Private => {
                let mut private_frames = BTreeMap::new();
                // 不需要连续的物理页
                for _ in 0..count {
                    let mut frame = alloc_frame(1).expect("Failed to allocate memory area: OOM");
                    frame.clear();
                    private_frames.insert(frame.addr(), frame);
                }
                Self {
                    name,
                    base_vaddr,
                    size: count * MMArch::PAGE_SIZE,
                    area_type,
                    permission,
                    private_frames,
                }
            }
            MemoryAreaType::Identical => Self {
                name,
                base_vaddr,
                size: count * MMArch::PAGE_SIZE,
                area_type,
                permission,
                private_frames: BTreeMap::new(),
            },
        }
    }

    /// 写入数据到内存区域的 `offset` 偏移处
    ///
    /// # Panics
    ///
    /// - 如果内存区域类型不为 [MemoryAreaType::Private]，则 panic
    /// - 如果 `offset` 加上 `data` 的长度大于内存区域的大小，则 panic
    pub fn write_data(&mut self, offset: usize, data: &[u8]) {
        assert!(
            self.area_type == MemoryAreaType::Private,
            "Memory area type is not private"
        );
        assert!(
            offset + data.len() <= self.size,
            "Data length is greater than memory area size"
        );
        let mut pos = 0;
        let mut frame_iter = self.private_frames.values_mut().enumerate();
        for block in BlockIterator::new(MMArch::PAGE_SIZE, offset, data.len()) {
            let (mut idx, mut frame) = frame_iter.next().unwrap();
            while idx != block.block_id() {
                (idx, frame) = frame_iter.next().unwrap();
            }
            let frame = frame.as_mut_slice();
            frame[block.offset()..block.offset() + block.size()]
                .copy_from_slice(&data[pos..pos + block.size()]);
            pos += block.size();
        }
    }
}

impl PartialEq for MemoryArea {
    fn eq(&self, other: &Self) -> bool {
        self.base_vaddr() == other.base_vaddr()
    }
}

/// 页表
pub struct MemorySpace {
    /// 根页表
    root_page_table: PhysAddr,
    /// 页表占用的物理页
    page_table_frames: BTreeMap<PhysAddr, PageFrame>,
    /// 内存区域
    areas: BTreeMap<VirtAddr, MemoryArea>,
}

type PTE = <MMArch as MemoryManagementArch>::PTE;

impl MemorySpace {
    /// 创建一个空白页表，对应的物理页帧属于当前内存空间
    ///
    /// 返回创建的页表的物理地址
    pub fn create_page_table(&mut self) -> PhysAddr {
        let count = MMArch::PAGE_TABLE_SIZE.div_ceil(MMArch::PAGE_SIZE);
        let mut frame = alloc_frame(count).expect("Failed to allocate page table: OOM");
        frame.clear();
        let paddr = frame.addr();
        assert!(!self.page_table_frames.contains_key(&frame.addr()));
        self.page_table_frames.insert(frame.addr(), frame);
        paddr
    }

    /// 创建内存空间
    ///
    /// 自动映射跳板区
    pub fn create() -> Self {
        let mut memory_space = Self {
            page_table_frames: BTreeMap::new(),
            root_page_table: PhysAddr::new(0),
            areas: BTreeMap::new(),
        };
        let root_page_table = memory_space.create_page_table();
        memory_space.root_page_table = root_page_table;
        memory_space.map_trampoline();

        memory_space
    }

    /// 映射跳板区域
    pub fn map_trampoline(&mut self) {
        unsafe extern "C" {
            fn strampoline();
            fn etrampoline();
        }
        let strampoline = strampoline as *const () as usize;
        let etrampoline = etrampoline as *const () as usize;
        assert!(etrampoline - strampoline == MMArch::TRAMPOLINE_PAGE_COUNT * MMArch::PAGE_SIZE);
        let start_vaddr = VirtAddr::new(
            (1 << MMArch::VADDR_BITS_COUNT) - MMArch::TRAMPOLINE_PAGE_COUNT * MMArch::PAGE_SIZE,
        );
        for i in 0..MMArch::TRAMPOLINE_PAGE_COUNT {
            let vaddr = start_vaddr + i * MMArch::PAGE_SIZE;
            let paddr = PhysAddr::new(strampoline + i * MMArch::PAGE_SIZE);
            self.map(
                vaddr,
                paddr,
                MemoryPermission::Readable | MemoryPermission::Executable,
            );
        }
    }

    /// 检查是否和已有 area 重叠
    fn check_area_overlap(&self, start: VirtAddr, size: usize) -> bool {
        if self.areas.contains_key(&start) {
            return true;
        }
        if let Some((_, pred)) = self.areas.range(..start).next_back()
            && pred.base_vaddr() + pred.size() > start
        {
            return true;
        }
        if let Some((nb, _)) = self
            .areas
            .range((Bound::Excluded(start), Bound::Unbounded))
            .next()
            && *nb < start + size
        {
            return true;
        }
        false
    }

    fn flush(&mut self) {
        MMArch::local_flush_tlb();
        // SAFETY: flush 仅在 push/remove 中被调用，而它们都通过 KERNEL_SPACE.lock() 等自旋锁进入，
        // 此时中断已经关闭
        unsafe { MMArch::tlb_shootdown() };
    }

    /// 添加一个内存区域
    pub fn push(&mut self, area: MemoryArea) {
        // 需要确保没重叠
        if core::hint::unlikely(self.check_area_overlap(area.base_vaddr(), area.size())) {
            panic!("Memory area {:?} overlaps with existing area", area);
        }
        match area.area_type() {
            MemoryAreaType::Private => {
                // 还需要把物理页映射到页表中
                for (idx, (_, frame)) in area.private_frame().iter().enumerate() {
                    let vaddr = area.base_vaddr() + idx * MMArch::PAGE_SIZE;
                    let paddr = frame.addr();
                    let permission = area.permission();
                    self.map(vaddr, paddr, permission);
                }
            }
            MemoryAreaType::Identical => {
                for i in 0..(area.size() / MMArch::PAGE_SIZE) {
                    let vaddr = area.base_vaddr() + i * MMArch::PAGE_SIZE;
                    let paddr = PhysAddr::new(vaddr.inner());
                    let permission = area.permission();
                    self.map(vaddr, paddr, permission);
                }
            }
        }
        self.areas.insert(area.base_vaddr(), area);
        self.flush();
    }

    /// # Safety
    ///
    /// - 需要确保引用 PageTable 的生命周期不超过 MemorySpace 的生命周期
    /// - 你应该把返回值视为一个对 MemorySpace 的可变引用，遵循可变引用的所有规则
    pub unsafe fn table(&self) -> PageTable {
        PageTable::new(
            VirtAddr::new(0),
            self.root_page_table,
            MMArch::PAGE_LEVELS - 1,
        )
    }

    /// 映射一个虚拟页面到物理页面
    ///
    /// # Parameters
    ///
    /// - `vaddr`: 虚拟页面起始地址
    /// - `paddr`: 物理页面起始地址
    /// - `flags`: 映射的属性
    /// - `frame_allocator`: 物理页帧分配器
    fn map(&mut self, vaddr: VirtAddr, paddr: PhysAddr, permission: MemoryPermission) {
        let pte = PTE::new_leaf(paddr, permission);
        let mut table = unsafe { self.table() };
        loop {
            let index = table
                .index_of(vaddr)
                .expect("Virtual address not in current page table");
            if table.level() == 0 {
                table.set(index, pte);
                break;
            } else {
                if let Some(next_table) = unsafe { table.next_level_table(index) } {
                    table = next_table;
                } else {
                    // 创建下一级页表
                    let next_table_paddr = self.create_page_table();
                    let next_table = PageTable::new(
                        table.entry_base(index),
                        next_table_paddr,
                        table.level() - 1,
                    );
                    let next_table_pte = PTE::new_non_leaf(next_table_paddr);
                    table.set(index, next_table_pte);
                    table = next_table;
                }
            }
        }
    }

    /// 取消映射一个虚拟页面
    fn unmap(&mut self, vaddr: VirtAddr) {
        let mut table = unsafe { self.table() };
        loop {
            let index = table
                .index_of(vaddr)
                .expect("Virtual address not in current page table");
            if table.level() == 0 {
                table.set(index, PTE::empty());
                break;
            } else {
                table = unsafe {
                    table
                        .next_level_table(index)
                        .expect("Next level table not found")
                };
            }
        }
    }

    /// 将从某个虚拟地址开始的内存区域移除
    ///
    /// # Panics
    ///
    /// 必须存在一个已经映射的内存区域的起始地址完全等于 `start_vaddr`，
    /// 否则会 panic
    pub fn remove(&mut self, start_vaddr: VirtAddr) {
        let area = self
            .areas
            .remove(&start_vaddr)
            .expect("No area found at start_vaddr");
        for i in 0..(area.size() / MMArch::PAGE_SIZE) {
            let vaddr = area.base_vaddr() + i * MMArch::PAGE_SIZE;
            self.unmap(vaddr);
        }
        self.flush();
    }

    /// 将当前内存空间的虚拟地址翻译成物理地址
    ///
    /// # Returns
    ///
    /// 如果当前内存空间没有映射该虚拟地址，则返回 None
    ///
    /// 否则返回对应的物理地址和权限
    pub fn translate_vaddr(&self, vaddr: VirtAddr) -> Option<(PhysAddr, MemoryPermission)> {
        let mut table = unsafe { self.table() };
        loop {
            let index = table.index_of(vaddr)?;
            if table.level() == 0 {
                let page_offset = vaddr.inner() & (MMArch::PAGE_SIZE - 1);
                let pte = table.get(index);
                if !pte.is_valid() {
                    return None;
                }
                return Some((pte.paddr() + page_offset, pte.permission()));
            } else {
                table = unsafe { table.next_level_table(index)? }
            }
        }
    }

    /// 打印当前内存空间的情况
    pub fn print_info(&self, show_page_table_frames: bool) {
        println!("Root Page Table: {:?}", self.root_page_table);
        if show_page_table_frames {
            for frame in self.page_table_frames.values() {
                println!("\t{:?}", frame);
            }
        }
        let name_col = self
            .areas
            .values()
            .map(|a| a.name().len())
            .max()
            .unwrap_or(0)
            .max(4);
        println!("Memory Areas:");
        for area in self.areas.values() {
            let area_type = match area.area_type() {
                MemoryAreaType::Private => "P",
                MemoryAreaType::Identical => "I",
            };
            println!(
                "  {:>1}  {:<6?}  {:<name_col$}  [0x{:X}, 0x{:X})",
                area_type,
                area.permission(),
                area.name(),
                area.base_vaddr().inner(),
                area.base_vaddr().inner() + area.size(),
                name_col = name_col,
            );
        }
    }

    pub fn activate(&self) {
        MMArch::activate(self);
    }
}
