use crate::{
    error::SystemError,
    mm::{address::VirtAddr, mem_space::MemoryPermission},
    process::ProcessManager,
};
use syscall_macros::syscall;

#[syscall(name = "SYS_WAITPID", id = 8)]
fn sys_waitpid(args: [usize; 6]) -> Result<usize, SystemError> {
    let pid = args[0];
    let status_ptr = args[1];
    let non_blocking = args[2] != 0;
    let process = ProcessManager::current_process();
    if pid != 0 {
        // 判断是否为子进程
        // 这里应该不会存在 TOCTOU 问题，如果一个进程现在是子进程，那么不可能后面又不是子进程了
        let process_inner = process.inner();
        if !process_inner.exited_children.contains_key(&pid)
            && !process_inner.children.contains_key(&pid)
        {
            return Err(SystemError::ECHILD);
        }
    }
    let try_wait = || {
        let mut inner = process.inner();
        if pid == 0 {
            // 任意子进程退出都 ok
            inner.exited_children.pop_first()
        } else {
            // 等待指定 pid 的子进程退出
            inner.exited_children.remove(&pid).map(|code| (pid, code))
        }
    };
    let (exited_pid, exited_code) = if non_blocking {
        let Some(res) = try_wait() else {
            return Ok(0);
        };
        res
    } else {
        process.wait_queue.wait_until(try_wait, true)?
    };
    if status_ptr != 0 {
        let process_inner = process.inner();
        let memory_space = process_inner.memory_space.as_ref().unwrap();
        // 返回值只是一个字节
        // 为了防止 TOCTOU 问题，所以这里是在要写入数据的时候进行权限的检查
        // TODO 如果检查失败的话，此时我们已经确确实实拿到了一个子进程的退出数据到 res 了，需要考虑把拿出来的这个数据再放回去。
        let ptr_permission = memory_space
            .check_permission(VirtAddr::new(status_ptr), VirtAddr::new(status_ptr + 1))?;
        if !ptr_permission.contains(MemoryPermission::UserAccessible)
            || !ptr_permission.contains(MemoryPermission::Writable)
        {
            return Err(SystemError::EFAULT);
        }
        memory_space.copyout(VirtAddr::new(status_ptr), &[exited_code]);
    }
    Ok(exited_pid)
}
