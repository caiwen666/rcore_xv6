use alloc::vec;
use syscall_macros::syscall;

use crate::{
    mm::{address::VirtAddr, mem_space::MemoryPermission},
    process::cpu::CPUManager,
};

#[syscall(name = "SYS_WRITE", id = 0)]
fn sys_write(args: [usize; 6]) -> isize {
    let fd: usize = args[0];
    let buf: VirtAddr = VirtAddr::new(args[1]);
    let len: usize = args[2];
    if len == 0 {
        return 0;
    }
    let task = CPUManager::current_task().unwrap();
    let resource = task.process_resource();
    let resource_guard = resource.lock();
    let Some(file) = resource_guard
        .fd_table
        .get(fd)
        .and_then(|f| f.as_ref().cloned())
    else {
        return -1;
    };
    let memory_space = resource_guard.memory_space.as_ref().unwrap();
    let Some(permission) = memory_space.check_permission(buf, buf + len) else {
        return -1;
    };
    if !permission.contains(MemoryPermission::UserAccessible)
        || !permission.contains(MemoryPermission::Readable)
    {
        return -1;
    }
    let mut kernel_buf = vec![0; len];
    memory_space.copyin(buf, &mut kernel_buf);
    drop(resource_guard);
    file.write(&kernel_buf).map(|v| v as isize).unwrap_or(-1)
}
