mod completion;
mod timer;
mod wait_queue;

use crate::{
    error::SystemError,
    process::{
        ProcessManager,
        schedule::TaskScheduler,
        task::{TaskControlBlock, TaskStatus},
    },
    sync::spin::SpinMutex,
};
use alloc::{
    rc::Rc,
    sync::{Arc, Weak},
};
use core::marker::PhantomData;

pub use completion::Completion;
pub use timer::{check_sleep_timer, sleep_with_interval};
pub use wait_queue::WaitQueue;

pub struct Waker {
    state: SpinMutex<WakerState>,
    target: Weak<TaskControlBlock>,
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum WakerState {
    Idle,
    Sleeping,
    Notified,
    Closed,
}

impl Waker {
    pub fn close(&self) {
        *self.state.lock() = WakerState::Closed;
    }

    /// 唤醒对应的 waiter
    ///
    /// 能正确处理提前唤醒、目标线程已经被杀死、重复唤醒的情况
    pub fn wake(&self) -> bool {
        let mut state = self.state.lock();
        match *state {
            WakerState::Closed | WakerState::Notified => false,
            WakerState::Idle => {
                *state = WakerState::Notified;
                true
            }
            WakerState::Sleeping => {
                *state = WakerState::Notified;
                if let Some(target) = self.target.upgrade() {
                    let mut task_inner = target.lock();
                    if let TaskStatus::Blocked(_) = task_inner.status {
                        task_inner.status = TaskStatus::Ready;
                        TaskScheduler::push(target.clone());
                    }
                }
                true
            }
        }
    }
}

pub struct Waiter {
    waker: Arc<Waker>,
    _nosend: PhantomData<Rc<()>>,
}

impl Drop for Waiter {
    fn drop(&mut self) {
        self.waker.close();
    }
}

impl Waiter {
    /// 创建一个 waiter/waker 对，其中的 waker 可以被 clone 多次分发到多个地方使用
    pub fn new_pair() -> (Self, Arc<Waker>) {
        let waker = Arc::new(Waker {
            state: SpinMutex::new(WakerState::Idle, "waker_state"),
            target: Arc::downgrade(&ProcessManager::current_task()),
        });
        let waiter = Self {
            waker: waker.clone(),
            _nosend: PhantomData,
        };
        (waiter, waker)
    }

    /// 将当前线程睡眠，该函数返回时必然是被 waker 唤醒或是被信号中断
    ///
    /// # Parameters
    ///
    /// - `interruptible`: 是否可以被信号中断，如果为 true 的话则该函数会抛出 [SystemError::EINTR] 错误，
    ///   为 false 的话该函数必然返回 Ok。
    ///
    /// # Panics
    ///
    /// 调用该函数时不能持有自旋锁，否则会 panic。
    ///
    /// # Errors
    ///
    /// [SystemError::EINTR] 当 `interruptible` 为 true 时，如果当前睡眠被信号中断，则返回该错误。
    pub fn wait(&self, interruptible: bool) -> Result<(), SystemError> {
        // loop 来防止潜在的虚假唤醒，确保 wait 返回一定是被 waker 唤醒或是被信号中断
        loop {
            let mut waker_state = self.waker.state.lock();
            match *waker_state {
                WakerState::Notified => {
                    *waker_state = WakerState::Idle;
                    return Ok(());
                }
                WakerState::Closed => return Ok(()),
                WakerState::Idle => {
                    *waker_state = WakerState::Sleeping;
                }
                _ => {}
            }

            let current_task = ProcessManager::current_task();
            let mut task_inner = current_task.lock();
            drop(waker_state);
            // 说明还没睡就被 kill 了
            if task_inner.killed {
                return Err(SystemError::EINTR);
            }
            task_inner.status = TaskStatus::Blocked(interruptible);
            task_inner = ProcessManager::go_scheduler(task_inner);
            // 因为被 kill 而唤醒
            if task_inner.killed {
                return Err(SystemError::EINTR);
            }
        }
    }
}
