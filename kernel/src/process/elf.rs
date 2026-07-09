use xmas_elf::{ElfFile, program::Type};

use crate::{
    arch::MMArch,
    error::SystemError,
    mm::{
        MemoryManagementArch,
        address::VirtAddr,
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission, MemorySpace},
    },
};

/// 解析并加载 elf 文件
///
/// # Return
///
/// 返回一个二元组：
///
/// - 第一个元素是基于 elf 文件创建的内存空间。该内存空间按照 elf 文件的 PT_LOAD 段创建内存区域，
///   同时还会映射跳板区域，除此之外不会再添加其他的内存区域
/// - 第二个元素是 elf 文件的 tls 区域的大小，如果不存在 tls 区域则为 None
/// - 第三个元素是 elf 文件的入口地址
///
/// # Errors
///
/// - [SystemError::ENOEXEC] 如果 elf 文件解析失败，则抛出该错误
pub fn load_elf(elf_data: &[u8]) -> Result<(MemorySpace, Option<usize>, VirtAddr), SystemError> {
    let elf = ElfFile::new(elf_data).map_err(|_| SystemError::ENOEXEC)?;
    // 解析 tls
    let tls_size = elf
        .program_iter()
        .find(|ph| ph.get_type().unwrap() == Type::Tls)
        .map(|ph| (ph.mem_size() as usize).div_ceil(MMArch::PAGE_SIZE) * MMArch::PAGE_SIZE);
    let mut memory_space = MemorySpace::create();
    for ph in elf.program_iter() {
        let ph_type = ph.get_type().map_err(|_| SystemError::ENOEXEC)?;
        if ph_type != Type::Load {
            continue;
        }
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
    }
    Ok((
        memory_space,
        tls_size,
        VirtAddr::new(elf.header.pt2.entry_point() as usize),
    ))
}
