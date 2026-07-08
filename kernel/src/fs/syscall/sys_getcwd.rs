use alloc::{string::String, vec::Vec};
use syscall_macros::syscall;

use crate::{error::SystemError, mm::address::VirtAddr, process::ProcessManager};

#[syscall(name = "SYS_GETCWD", id = 4)]
fn sys_getcwd(args: [usize; 6]) -> Result<usize, SystemError> {
    let path_addr = VirtAddr::new(args[0]);
    let max_len = args[1];
    if max_len == 0 {
        return Err(SystemError::EINVAL);
    }

    let process = ProcessManager::current_process();

    let mut inode = process.cwd();
    let mut path_inode = Vec::new();
    path_inode.push(inode.clone());
    while let Some(parent_inode) = inode.parent() {
        path_inode.push(parent_inode.clone());
        inode = parent_inode;
    }

    let mut path = String::new();
    for inode in path_inode.into_iter().rev().skip(1) {
        let dir_name = inode.dir_name().ok_or(SystemError::ENOENT)?;
        path.push('/');
        path.push_str(dir_name.as_str());
    }
    if path.is_empty() {
        path.push('/');
    }
    if path.len() >= max_len {
        return Err(SystemError::ERANGE);
    }

    let inner = process.inner();
    let memory_space = inner.memory_space.as_ref().unwrap();
    memory_space.copyout_str(path_addr, path)?;

    Ok(args[0])
}
