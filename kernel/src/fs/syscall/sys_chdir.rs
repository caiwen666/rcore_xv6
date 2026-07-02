use syscall_macros::syscall;

use crate::{
    error::SystemError,
    fs::vfs::{interface::FileType, lookup},
    mm::{address::VirtAddr, mem_space::MemoryPermission},
    process::cpu::CPUManager,
};

#[syscall(name = "SYS_CHDIR", id = 2)]
fn sys_chdir(args: [usize; 6]) -> Result<usize, SystemError> {
    const MAXPATH: usize = 128;
    let path_addr = VirtAddr::new(args[0]);

    let task = CPUManager::current_task().unwrap();
    let resource = task.process_resource();
    let resource_guard = resource.lock();
    let memory_space = resource_guard.memory_space.as_ref().unwrap();
    let path = memory_space.copyin_str(path_addr, MAXPATH)?;
    if path.is_empty() {
        return Err(SystemError::EINVAL);
    }
    let permission = memory_space.check_permission(path_addr, path_addr + path.len())?;
    if !permission.contains(MemoryPermission::UserAccessible)
        || !permission.contains(MemoryPermission::Readable)
    {
        return Err(SystemError::EFAULT);
    }
    drop(resource_guard);

    let cwd = resource.cwd();
    let inode = lookup(cwd, path.as_str()).ok_or(SystemError::ENOENT)?;
    if inode.metadata().file_type != FileType::Directory {
        return Err(SystemError::ENOTDIR);
    }
    let mut resource_guard = resource.lock();
    resource_guard.cwd.replace(inode);
    Ok(0)
}
