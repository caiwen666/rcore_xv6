use syscall_macros::syscall;

use crate::{
    error::SystemError,
    process::{ProcessManager, ProcessStatus, schedule::TaskScheduler, task::TaskStatus},
};

#[syscall(name = "SYS_EXIT", id = 5)]
fn sys_exit(args: [usize; 6]) -> Result<usize, SystemError> {
    let code = args[0] as u8;
    let process = ProcessManager::current_process();
    let mut inner = process.inner();
    // 之前已经有线程设置退出了，
    if matches!(inner.status, ProcessStatus::Exiting(_)) {
        return Ok(0);
    }
    // 先对所有线程发送 kill 信号并唤醒所有可中断的线程
    for (_, task) in inner.tasks.iter() {
        let Some(task) = task.upgrade() else {
            // 说明线程实际上已经退出了，但是还没有更新进程的线程列表
            continue;
        };
        let mut task_inner = task.lock();
        task_inner.killed = true;
        if task_inner.status == TaskStatus::Blocked(true) {
            TaskScheduler::push(task.clone());
        }
    }
    inner.status = ProcessStatus::Exiting(code);
    // 尽管这里返回了，但是我们在前面其实也向当前进程发送 killed 信号了，
    // 所以会在回到用户态之前退出线程，然后最后一个退出的线程负责整个进程的退出
    Ok(0)
}
