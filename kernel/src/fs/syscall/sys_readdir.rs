use alloc::vec;
use syscall_macros::syscall;

use crate::{
    error::SystemError,
    fs::{file::ReadDirControl, vfs::interface::FileType},
    mm::address::VirtAddr,
    process::ProcessManager,
};

const DT_DIR: u8 = 4;
const DT_REG: u8 = 8;

/// 一条变长 dirent 固定头部的大小（不含 d_name）：
/// d_ino(8) + d_off(8) + d_reclen(2) + d_type(1) = 19
const DIRENT_HEADER_LEN: usize = 19;

/// 计算一条变长 dirent 上对齐到 8 字节边界后的总长度
fn dirent_reclen(name_len: usize) -> usize {
    // +1 是结尾的 NUL
    let raw = DIRENT_HEADER_LEN + name_len + 1;
    (raw + 7) & !7
}

#[syscall(name = "SYS_READDIR", id = 12)]
fn sys_readdir(args: [usize; 6]) -> Result<usize, SystemError> {
    let fd = args[0];
    let buf_vaddr = VirtAddr::new(args[1]);
    let count = args[2];
    // 缓冲区连一条最短的 dirent 都放不下
    if count < dirent_reclen(1) {
        return Err(SystemError::EINVAL);
    }

    let process = ProcessManager::current_process();
    let inner = process.inner();
    let file = inner.fd_table.get(fd).cloned().ok_or(SystemError::EBADF)?;
    drop(inner);

    let mut kernel_buf = vec![0u8; count];
    let mut pos: usize = 0;
    // 第一条目录项就因名字过长而放不下时置位，稍后返回 EINVAL
    let mut too_small = false;

    // 回调只写入栈上的局部变量，不额外加锁；copyout 等 read_dir 返回后再做
    file.read_dir(&mut |entry, control| {
        let name = entry.name.as_bytes();
        let reclen = dirent_reclen(name.len());
        if pos + reclen > count {
            // 放不下了：不推进 offset，下次接着读这一条
            if pos == 0 {
                too_small = true;
            }
            *control = ReadDirControl::StopWithBackroll;
            return Ok(());
        }
        let d_type = match entry.file_type {
            FileType::File => DT_REG,
            FileType::Directory => DT_DIR,
        };
        // 写入 dirent；padding 部分（NUL 到 reclen 之间）已经是 0
        kernel_buf[pos..pos + 8].copy_from_slice(&entry.inode.to_le_bytes());
        kernel_buf[pos + 8..pos + 16].copy_from_slice(&entry.offset_cookie.to_le_bytes());
        kernel_buf[pos + 16..pos + 18].copy_from_slice(&(reclen as u16).to_le_bytes());
        kernel_buf[pos + 18] = d_type;
        kernel_buf[pos + DIRENT_HEADER_LEN..pos + DIRENT_HEADER_LEN + name.len()]
            .copy_from_slice(name);
        kernel_buf[pos + DIRENT_HEADER_LEN + name.len()] = 0; // NUL
        pos += reclen;
        *control = ReadDirControl::Continue;
        Ok(())
    })?;

    if too_small {
        // 进了回调但一条都没写下：缓冲区太小
        return Err(SystemError::EINVAL);
    }

    let inner = process.inner();
    let memory_space = inner.memory_space.as_ref().unwrap();
    memory_space.copyout_bytes(buf_vaddr, &kernel_buf[..pos])?;
    Ok(pos)
}
