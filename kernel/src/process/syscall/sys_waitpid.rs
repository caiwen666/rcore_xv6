use crate::{error::SystemError, mm::address::VirtAddr, process::ProcessManager};
use syscall_macros::syscall;

#[syscall(name = "SYS_WAITPID", id = 8)]
fn sys_waitpid(args: [usize; 6]) -> Result<usize, SystemError> {
    let pid = args[0];
    // 32 位整数指针
    let status_ptr = VirtAddr::new(args[1]);
    let non_blocking = args[2] != 0;
    let process = ProcessManager::current_process();
    let process_inner = process.inner();
    if pid != 0 {
        // 判断是否为子进程
        // 这里应该不会存在 TOCTOU 问题，如果一个进程现在是子进程，那么不可能后面又不是子进程了
        if !process_inner.exited_children.contains_key(&pid)
            && !process_inner.children.contains_key(&pid)
        {
            return Err(SystemError::ECHILD);
        }
    } else {
        // 如果不指定 pid，那么也需要确保当前进程有子进程，否则会一直阻塞
        if process_inner.children.is_empty() && process_inner.exited_children.is_empty() {
            return Err(SystemError::ECHILD);
        }
    }
    drop(process_inner);
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
    if !status_ptr.is_null() {
        let process_inner = process.inner();
        let memory_space = process_inner.memory_space.as_ref().unwrap();
        // 如果这里 copyout 失败的话，我们已经成功取出一个已经退出的进程的返回值了，
        // 这意味着这个返回值可能会丢失，但我们不去做回滚什么的了，反正 Linux 也是这么干的。
        memory_space.copyout(status_ptr, exited_code as u32)?;
    }
    Ok(exited_pid)
}
