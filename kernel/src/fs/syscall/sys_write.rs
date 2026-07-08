use alloc::vec;
use syscall_macros::syscall;

use crate::{
    error::SystemError,
    mm::address::VirtAddr,
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
    let file = inner.fd_table.get(fd).cloned().ok_or(SystemError::EBADF)?;
    let mut kernel_buf = vec![0; len];
    let memory_space = inner.memory_space.as_ref().unwrap();
    memory_space.copyin_bytes(buf, &mut kernel_buf)?;
    drop(inner);
    file.write(&kernel_buf)
}
