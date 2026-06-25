use crate::{
    arch::MMArch,
    mm::{
        MemoryManagementArch,
        address::VirtAddr,
        mem_space::{MemoryPermission, MemorySpace},
    },
    utils::BlockIterator,
};

impl MemorySpace {
    /// 检查地址空间从 `from` 到 `to`（左闭右开）这个区间内所有页表项权限的交集。
    /// 如果这个区间存在未映射的部分，则返回 None。
    ///
    /// # Panics
    ///
    /// 如果 `from` 大于等于 `to`，则 panic
    pub fn check_permission(&self, from: VirtAddr, to: VirtAddr) -> Option<MemoryPermission> {
        assert!(
            from < to,
            "from must be less than to, got from = {:?}, to = {:?}",
            from,
            to
        );

        let mut permission = None;
        let mut vaddr = from;

        for block in BlockIterator::new(MMArch::PAGE_SIZE, from.inner(), to.inner() - from.inner())
        {
            let (_, page_permission) = self.translate_vaddr(vaddr)?;
            vaddr += block.size();
            let Some(permission) = permission.as_mut() else {
                permission = Some(page_permission);
                continue;
            };
            *permission &= page_permission;
        }

        permission
    }

    /// 将内存空间中虚拟地址为 `vaddr` 处开始的数据拷贝到 `buf` 中
    ///
    /// # Panics
    ///
    /// 如果拷贝过程中发现有当前内存空间没有映射的地方，则 panic
    ///
    /// # Notes
    ///
    /// 建议调用前使用 [MemorySpace::check_permission] 检查权限和是否存在映射
    pub fn copyin(&self, mut vaddr: VirtAddr, buf: &mut [u8]) {
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), buf.len()) {
            let (paddr, _) = self
                .translate_vaddr(vaddr)
                .expect("copyin: vaddr not mapped");
            buf[pos..pos + block.size()].copy_from_slice(paddr.as_slice(block.size()));
            vaddr += block.size();
            pos += block.size();
        }
    }
}
