use alloc::vec;
use syscall_macros::syscall;

use crate::{
    error::SystemError,
    mm::{
        address::VirtAddr,
        mem_space::{MemoryPermission, MemorySpace},
    },
    process::ProcessManager,
};

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

    let check = |memory_space: &MemorySpace| {
        let permission = memory_space.check_permission(buf, buf + len)?;
        if !permission.contains(MemoryPermission::UserAccessible)
            || !permission.contains(MemoryPermission::Writable)
        {
            return Err(SystemError::EFAULT);
        }
        Ok(())
    };

    let memory_space = inner.memory_space.as_ref().unwrap();
    // 先检查一遍
    check(memory_space)?;
    // 后面的 file.read 会堵塞，这里先把锁给释放
    drop(inner);

    let mut kernel_buf = vec![0; len];
    let len = file.read(&mut kernel_buf)?;

    let inner = process.inner();
    let memory_space = inner.memory_space.as_ref().unwrap();
    // 再检查一遍
    // 如果再检查一遍发现不满足要求的话，我们会白白消耗掉 file 的数据（如果 file 是字节流类型的）
    // 这种情况说明是用户程序的问题，我们内核暂时不去多管
    check(memory_space)?;
    memory_space.copyout(buf, &kernel_buf);
    Ok(len)
}
