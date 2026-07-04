use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use crate::{
    error::SystemError,
    process::sleep::{Waiter, Waker},
    sync::spin::SpinMutex,
};

/// 等待队列，用于将多个线程阻塞在同一个条件上
pub struct WaitQueue {
    inner: SpinMutex<WaitQueueInner>,
}

struct WaitQueueInner {
    waiters: VecDeque<Arc<Waker>>,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            inner: SpinMutex::new(
                WaitQueueInner {
                    waiters: VecDeque::new(),
                },
                "wait_queue",
            ),
        }
    }

    fn register_waker(&self, waker: Arc<Waker>) {
        let mut inner = self.inner.lock();
        inner.waiters.push_back(waker);
    }

    fn remove_waker(&self, target: &Arc<Waker>) {
        let mut inner = self.inner.lock();
        inner.waiters.retain(|w| !Arc::ptr_eq(w, target));
    }

    /// 阻塞当前线程，将当前线程放到队列中等待，直到条件满足或是被信号中断。
    ///
    /// # Parameters
    ///
    /// - `cond`: 条件闭包，该闭包会被多次调用，每次调用时，如果闭包返回 None 则表示条件
    ///   不满足，需要继续等待。返回 Some(R) 则表示条件满足，该函数会把 R 返回。
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
    pub fn wait_until<F, R>(&self, mut cond: F, interruptible: bool) -> Result<R, SystemError>
    where
        F: FnMut() -> Option<R>,
    {
        if let Some(res) = cond() {
            return Ok(res);
        }
        let (waiter, waker) = Waiter::new_pair();

        let cancel = |err: SystemError, cond: &mut F| {
            waker.close();
            if let Some(res) = cond() {
                return Ok(res);
            }
            Err(err)
        };

        loop {
            self.register_waker(waker.clone());
            if let Some(res) = cond() {
                self.remove_waker(&waker);
                return Ok(res);
            }
            if let Err(e) = waiter.wait(interruptible) {
                self.remove_waker(&waker);
                return cancel(e, &mut cond);
            }
        }
    }

    /// 唤醒所有等待者
    pub fn wake_all(&self) {
        let mut inner = self.inner.lock();
        for waker in inner.waiters.drain(..) {
            waker.wake();
        }
    }
}
