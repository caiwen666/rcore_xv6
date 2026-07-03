use crate::{
    process::{
        ProcessManager,
        schedule::TaskScheduler,
        task::{TaskControlBlock, TaskStatus},
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
};
use alloc::{collections::vec_deque::VecDeque, sync::Arc};

pub struct Condvar {
    queue: SpinMutex<VecDeque<Arc<TaskControlBlock>>>,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            queue: SpinMutex::new(VecDeque::new(), "condvar_queue"),
        }
    }

    /// 将当前线程休眠，直到被唤醒
    ///
    /// # Panics
    ///
    /// 调用者需要保证调用时只持有 guard 对应的自旋锁，否则持有的其他锁可能会死锁，
    /// 如果调用者未满足该条件，则 panic
    pub fn wait<'a, T>(&self, guard: SpinMutexGuard<'a, T>) -> SpinMutexGuard<'a, T> {
        let current_task = ProcessManager::current_task();

        let mut queue = self.queue.lock();
        // 已经给 queue 加上锁了，所以可以直接释放 guard
        let lock = guard.lock();
        unsafe { lock.unlock() };
        // 将当前任务加入到队列中
        queue.push_back(current_task.clone());
        let mut task_inner = current_task.lock();
        // 已经把当前任务放入队列了，所以对队列的锁可以释放掉
        // 由于我们目前还持有对当前线程的锁，所以 wakeup 时会等待我们这里的 wait 操作完全回到调度循环
        drop(queue);

        task_inner.status = TaskStatus::Blocked;
        ProcessManager::go_scheduler(task_inner);

        // 之前我们将 guard 对应的锁释放了，现在我们重新加锁
        let mut unused_guard = lock.lock();
        // 但是我们想把原来的 guard 返回，确保生命周期一致，所以这里的 unused_guard leak 掉
        unsafe { unused_guard.leak() };
        drop(unused_guard);

        guard
    }

    pub fn notify_all(&self) {
        let mut queue = self.queue.lock();
        while let Some(task) = queue.pop_front() {
            let mut task_inner = task.lock();
            task_inner.status = TaskStatus::Ready;
            TaskScheduler::push(task.clone());
        }
    }
}
