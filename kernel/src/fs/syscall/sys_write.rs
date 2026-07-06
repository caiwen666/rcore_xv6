use alloc::vec;
use syscall_macros::syscall;

use crate::{
    error::SystemError,
    mm::{address::VirtAddr, mem_space::MemoryPermission},
    process::ProcessManager,
};

#[syscall(name = "SYS_WRITE", id = 0)]
fn sys_write(args: [usize; 6]) -> Result<usize, SystemError> {
    let fd = args[0];
    let buf = VirtAddr::new(args[1]);
    let len = args[2];
    if len == 0 {
        return Ok(0);
    }
    let process = ProcessManager::current_process();
    let inner = process.inner();
    let file = inner
        .fd_table
        .get(fd)
        .and_then(|f| f.as_ref().cloned())
        .ok_or(SystemError::EBADF)?;
    let memory_space = inner.memory_space.as_ref().unwrap();
    let permission = memory_space.check_permission(buf, buf + len)?;
    if !permission.contains(MemoryPermission::UserAccessible)
        || !permission.contains(MemoryPermission::Readable)
    {
        return Err(SystemError::EFAULT);
    }
    let mut kernel_buf = vec![0; len];
    memory_space.copyin(buf, &mut kernel_buf);
    drop(inner);
    file.write(&kernel_buf)
}
