use alloc::string::String;

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

    /// 将 `buf` 中的数据拷贝到内存空间中虚拟地址为 `vaddr` 处开始的位置
    ///
    /// # Panics
    ///
    /// 如果拷贝过程中发现有当前内存空间没有映射的地方，则 panic
    ///
    /// # Notes
    ///
    /// 建议调用前使用 [MemorySpace::check_permission] 检查权限和是否存在映射
    pub fn copyout(&self, mut vaddr: VirtAddr, buf: &[u8]) {
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), buf.len()) {
            let (paddr, _) = self
                .translate_vaddr(vaddr)
                .expect("copyin: vaddr not mapped");
            paddr
                .as_slice_mut(block.size())
                .copy_from_slice(&buf[pos..pos + block.size()]);
            vaddr += block.size();
            pos += block.size();
        }
    }

    /// 将内存空间中虚拟地址为 `vaddr` 处开始的字符串拷贝出来
    ///
    /// # Returns
    ///
    /// 当遇到 `\0` 字符，或是长度达到 `max_len`，或是遇到了未被映射的内存区域，则返回
    pub fn copyin_str(&self, mut vaddr: VirtAddr, max_len: usize) -> String {
        if max_len == 0 {
            return String::new();
        }
        let mut s = String::new();
        for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), max_len) {
            let Some((paddr, _)) = self.translate_vaddr(vaddr) else {
                break;
            };
            let slice = paddr.as_slice(block.size());
            for c in slice {
                if *c == 0 {
                    break;
                }
                s.push(*c as char);
                if s.len() == max_len {
                    break;
                }
            }
            vaddr += block.size();
        }
        s
    }

    /// 将字符串拷贝到内存空间中虚拟地址为 `vaddr` 处开始的位置
    ///
    /// # Panics
    ///
    /// 如果拷贝过程中发现有当前内存空间没有映射的地方，则 panic
    ///
    /// # Notes
    ///
    /// 建议调用前使用 [MemorySpace::check_permission] 检查权限和是否存在映射
    pub fn copyout_str(&self, vaddr: VirtAddr, mut s: String) {
        s.push('\0');
        self.copyout(vaddr, s.as_bytes());
    }
}
