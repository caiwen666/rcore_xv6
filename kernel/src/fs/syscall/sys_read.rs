use alloc::vec;
use syscall_macros::syscall;

use crate::{error::SystemError, mm::address::VirtAddr, process::ProcessManager};

#[syscall(name = "SYS_READ", id = 1)]
fn sys_read(args: [usize; 6]) -> Result<usize, SystemError> {
    let fd = args[0];
    let buf = VirtAddr::new(args[1]);
    let len = args[2];
    if len == 0 {
        return Ok(0);
    }
    let process = ProcessManager::current_process();
    let inner = process.inner();
    let file = inner.fd_table.get(fd).cloned().ok_or(SystemError::EBADF)?;
    // 后面的 file.read 会堵塞，这里先把锁给释放
    drop(inner);

    let mut kernel_buf = vec![0; len];
    let len = file.read(&mut kernel_buf)?;

    let inner = process.inner();
    let memory_space = inner.memory_space.as_ref().unwrap();
    memory_space.copyout_bytes(buf, &kernel_buf)?;
    Ok(len)
}
