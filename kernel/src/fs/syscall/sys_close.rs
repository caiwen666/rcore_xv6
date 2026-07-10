use syscall_macros::syscall;

use crate::{error::SystemError, process::ProcessManager};

#[syscall(name = "SYS_CLOSE", id = 11)]
fn sys_close(args: [usize; 6]) -> Result<usize, SystemError> {
    let fd = args[0];
    let process = ProcessManager::current_process();
    let mut inner = process.inner();
    if inner.fd_table.get(fd).is_none() {
        return Err(SystemError::EBADF);
    }
    inner.fd_table.pop(fd);
    Ok(0)
}
