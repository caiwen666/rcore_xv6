use syscall_macros::syscall;

use crate::{error::SystemError, mm::address::VirtAddr, process::ProcessManager};

#[syscall(name = "SYS_OPEN", id = 10)]
fn sys_open(args: [usize; 6]) -> Result<usize, SystemError> {
    const MAXPATH: usize = 128;
    let path_address = VirtAddr::new(args[0]);

    let process = ProcessManager::current_process();
    let inner = process.inner();
    let memory_space = inner.memory_space.as_ref().unwrap();
    let path = memory_space.copyin_str(path_address, MAXPATH)?;
    if path.is_empty() {
        return Err(SystemError::EINVAL);
    }
    drop(inner);

    // open_file 会再次对 inner 加锁，这里先释放
    let fd = process.open_file(&path).ok_or(SystemError::ENOENT)?;
    Ok(fd as usize)
}
