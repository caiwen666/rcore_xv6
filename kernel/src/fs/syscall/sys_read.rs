use alloc::vec;
use syscall_macros::syscall;

use crate::{
    mm::{
        address::VirtAddr,
        mem_space::{MemoryPermission, MemorySpace},
    },
    process::cpu::CPUManager,
};

#[syscall(name = "SYS_READ", id = 1)]
fn sys_read(args: [usize; 6]) -> isize {
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

    let check = |memory_space: &MemorySpace| {
        let Some(permission) = memory_space.check_permission(buf, buf + len) else {
            return false;
        };
        if !permission.contains(MemoryPermission::UserAccessible)
            || !permission.contains(MemoryPermission::Writable)
        {
            return false;
        }
        true
    };

    let memory_space = resource_guard.memory_space.as_ref().unwrap();
    // 先检查一遍
    if !check(memory_space) {
        return -1;
    }
    // 后面的 file.read 会堵塞，这里先把锁给释放
    drop(resource_guard);

    let mut kernel_buf = vec![0; len];
    let Some(len) = file.read(&mut kernel_buf) else {
        return -1;
    };

    let resource_guard = resource.lock();
    let memory_space = resource_guard.memory_space.as_ref().unwrap();
    // 再检查一遍
    if !check(memory_space) {
        // 如果再检查一遍发现不满足要求的话，我们会白白消耗掉 file 的数据（如果 file 是字节流类型的）
        // 这种情况说明是用户程序的问题，我们内核暂时不去多管
        return -1;
    }
    memory_space.copyout(buf, &kernel_buf);
    len as isize
}
