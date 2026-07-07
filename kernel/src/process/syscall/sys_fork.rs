use syscall_macros::syscall;

use crate::{
    error::SystemError,
    process::{ProcessManager, context::TrapContext},
};

#[syscall(name = "SYS_FORK", id = 6)]
fn sys_fork(_args: [usize; 6]) -> Result<usize, SystemError> {
    let task = ProcessManager::current_task();
    // 先把返回值清零，这样在 fork 后，子进程会返回 0
    task.trap_context().set_return_value(0);
    let new_process = ProcessManager::fork()?;
    Ok(new_process.pid)
}
