use crate::{
    arch::MMArch,
    error::SystemError,
    mm::{
        MemoryManagementArch,
        address::VirtAddr,
        mem_space::{MemoryPermission, MemorySpace},
    },
    utils::BlockIterator,
};
use alloc::string::String;
use alloc::vec;
use zerocopy::{FromBytes, Immutable, IntoBytes};

impl MemorySpace {
    /// 检查地址空间从 `from` 到 `to`（左闭右开）这个区间内所有页表项权限的交集。
    /// 如果这个区间存在未映射的部分，则返回 None。
    ///
    /// # Panics
    ///
    /// 如果 `from` 大于等于 `to`，则 panic
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果区间内存在未映射的内存区域，则抛出该错误
    pub fn check_permission(
        &self,
        from: VirtAddr,
        to: VirtAddr,
    ) -> Result<MemoryPermission, SystemError> {
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
            let (_, page_permission) = self.translate_vaddr(vaddr).ok_or(SystemError::EFAULT)?;
            vaddr += block.size();
            let Some(permission) = permission.as_mut() else {
                permission = Some(page_permission);
                continue;
            };
            *permission &= page_permission;
        }

        Ok(permission.unwrap())
    }
    /// 将内存空间中虚拟地址为 `vaddr` 处开始的数据拷贝到 `buf` 中
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Readable] 权限，
    ///   则抛出该错误
    pub fn copyin_bytes(&self, mut vaddr: VirtAddr, buf: &mut [u8]) -> Result<(), SystemError> {
        if buf.is_empty() {
            return Ok(());
        }
        // 先整个检查一遍，然后再写，不能边写边检查，否则如果后面检查出错的话就会是只写了一半了
        let permission = self.check_permission(vaddr, vaddr + buf.len())?;
        if !permission.contains(MemoryPermission::UserAccessible)
            || !permission.contains(MemoryPermission::Readable)
        {
            return Err(SystemError::EFAULT);
        }
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), buf.len()) {
            let (paddr, _) = self.translate_vaddr(vaddr).ok_or(SystemError::EFAULT)?;
            buf[pos..pos + block.size()].copy_from_slice(paddr.as_slice(block.size()));
            vaddr += block.size();
            pos += block.size();
        }
        Ok(())
    }
    /// 将 `buf` 中的数据拷贝到内存空间中虚拟地址为 `vaddr` 处开始的位置
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Writable] 权限，
    ///   则抛出该错误
    pub fn copyout_bytes(&self, mut vaddr: VirtAddr, buf: &[u8]) -> Result<(), SystemError> {
        if buf.is_empty() {
            return Ok(());
        }
        // 先整个检查一遍，然后再写，不能边写边检查，否则如果后面检查出错的话就会是只写了一半了
        let permission = self.check_permission(vaddr, vaddr + buf.len())?;
        if !permission.contains(MemoryPermission::UserAccessible)
            || !permission.contains(MemoryPermission::Writable)
        {
            return Err(SystemError::EFAULT);
        }
        let mut pos = 0;
        for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), buf.len()) {
            let (paddr, _) = self.translate_vaddr(vaddr).ok_or(SystemError::EFAULT)?;
            paddr
                .as_slice_mut(block.size())
                .copy_from_slice(&buf[pos..pos + block.size()]);
            vaddr += block.size();
            pos += block.size();
        }
        Ok(())
    }

    /// 将内存空间中虚拟地址为 `vaddr` 处开始的字符串拷贝出来
    ///
    /// # Panics
    ///
    /// - 如果 `max_len` 为 0，则 panic
    ///
    /// # Errors
    ///
    /// - [SystemError::ENAMETOOLONG] 如果字符串长度超过 `max_len` 仍未遇到 `\0` 字符，则抛出该错误
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Readable] 权限，
    ///   则抛出该错误
    pub fn copyin_str(&self, mut vaddr: VirtAddr, max_len: usize) -> Result<String, SystemError> {
        assert!(max_len > 0, "max_len must be greater than 0");
        let mut s = String::new();
        'outer: for block in BlockIterator::new(MMArch::PAGE_SIZE, vaddr.inner(), max_len) {
            let (paddr, permission) = self.translate_vaddr(vaddr).ok_or(SystemError::EFAULT)?;
            if !permission.contains(MemoryPermission::UserAccessible)
                || !permission.contains(MemoryPermission::Readable)
            {
                return Err(SystemError::EFAULT);
            };
            let slice = paddr.as_slice(block.size());
            for c in slice {
                if *c == 0 {
                    break 'outer;
                }
                s.push(*c as char);
                if s.len() == max_len {
                    return Err(SystemError::ENAMETOOLONG);
                }
            }
            vaddr += block.size();
        }
        Ok(s)
    }

    /// 将字符串拷贝到内存空间中虚拟地址为 `vaddr` 处开始的位置
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Writable] 权限，
    ///   则抛出该错误
    pub fn copyout_str(&self, vaddr: VirtAddr, mut s: String) -> Result<(), SystemError> {
        s.push('\0');
        self.copyout_bytes(vaddr, s.as_bytes())
    }

    /// 将内存空间中虚拟地址为 `vaddr` 处开始的某个类型的数据拷贝出来
    ///
    /// 类型 T 应是 `#[repr(C)]` 的
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Readable] 权限，
    ///   则抛出该错误
    #[expect(unused)]
    pub fn copyin<T: FromBytes>(&self, vaddr: VirtAddr) -> Result<T, SystemError> {
        let mut buf = vec![0u8; core::mem::size_of::<T>()];
        self.copyin_bytes(vaddr, &mut buf)?;
        Ok(T::read_from_bytes(&buf).unwrap())
    }

    /// 将某个类型的数据拷贝到内存空间中虚拟地址为 `vaddr` 处开始的位置
    ///
    /// 类型 T 应是 `#[repr(C)]` 的
    ///
    /// # Errors
    ///
    /// - [SystemError::EFAULT] 如果内存空间中对应的区域没有映射
    ///   或是没有 [MemoryPermission::UserAccessible] 和 [MemoryPermission::Writable] 权限，
    ///   则抛出该错误
    pub fn copyout<T: IntoBytes + Immutable>(
        &self,
        vaddr: VirtAddr,
        data: T,
    ) -> Result<(), SystemError> {
        self.copyout_bytes(vaddr, data.as_bytes())
    }
}
