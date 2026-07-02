use alloc::{string::String, vec::Vec};
use syscall_macros::syscall;

use crate::{
    error::SystemError,
    mm::{address::VirtAddr, mem_space::MemoryPermission},
    process::ProcessManager,
};

#[syscall(name = "SYS_GETCWD", id = 4)]
fn sys_getcwd(args: [usize; 6]) -> Result<usize, SystemError> {
    let path_addr = VirtAddr::new(args[0]);
    let max_len = args[1];
    if max_len == 0 {
        return Err(SystemError::EINVAL);
    }

    let task = ProcessManager::current_task();
    let resource = task.process_resource();

    let mut inode = resource.cwd();
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

    let resource_guard = resource.lock();
    let memory_space = resource_guard.memory_space.as_ref().unwrap();
    let permission = memory_space.check_permission(path_addr, path_addr + path.len() + 1)?;
    if !permission.contains(MemoryPermission::UserAccessible)
        || !permission.contains(MemoryPermission::Writable)
    {
        return Err(SystemError::EFAULT);
    }
    memory_space.copyout_str(path_addr, path);

    Ok(args[0])
}
