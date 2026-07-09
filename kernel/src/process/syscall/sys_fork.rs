use crate::{
    error::SystemError,
    process::{
        ProcessControlBlock, ProcessControlBlockInner, ProcessManager, ProcessStatus,
        context::TrapContext, schedule::TaskScheduler, sleep::WaitQueue,
    },
    sync::spin::SpinMutex,
    utils::RecycleAllocator,
};
use alloc::collections::btree_map::BTreeMap;
use core::cell::SyncUnsafeCell;
use syscall_macros::syscall;

#[syscall(name = "SYS_FORK", id = 6)]
fn sys_fork(_args: [usize; 6]) -> Result<usize, SystemError> {
    let task = ProcessManager::current_task();
    // 先把返回值清零，这样在 fork 后，子进程会返回 0
    unsafe {
        task.trap_context().set_return_value(0);
    }
    let process = ProcessManager::current_process();
    let mut process_inner = process.inner();
    if !matches!(process_inner.status, ProcessStatus::Running) {
        // 如果当前进程已经退出，则直接抛出错误，这样当前系统调用会尽快返回，并在返回用户态前结束
        // 随便返回一个错误，因为什么错误其实都无所谓了，我们的目标是让系统调用尽快结束
        return Err(SystemError::EPERM);
    }
    if process_inner.tasks.len() != 1 {
        return Err(SystemError::EPERM);
    }
    let new_memory_space = process_inner.memory_space.as_ref().unwrap().fork();
    let new_process = ProcessManager::register(|pid| ProcessControlBlock {
        pid,
        tls_size: SyncUnsafeCell::new(process.tls_size()),
        wait_queue: WaitQueue::new(),
        inner: SpinMutex::new(
            ProcessControlBlockInner {
                status: ProcessStatus::Running,
                memory_space: Some(new_memory_space),
                tasks: RecycleAllocator::new(),
                cwd: process_inner.cwd.clone(),
                fd_table: process_inner.fd_table.clone(),
                heap_size: process_inner.heap_size,
                parent: Some(process.clone()),
                children: BTreeMap::new(),
                exited_children: BTreeMap::new(),
            },
            "process_inner",
        ),
    });
    let current_task = ProcessManager::current_task();
    let task = current_task.fork(new_process.clone());
    process_inner
        .children
        .insert(new_process.pid, new_process.clone());
    TaskScheduler::push(task);
    Ok(new_process.pid)
}
