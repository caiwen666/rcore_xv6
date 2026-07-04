use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use crate::{
    error::SystemError,
    process::sleep::{Waiter, Waker},
    sync::spin::SpinMutex,
};

/// 完成量，用于多个线程等待一个操作完成。
///
/// 完成这个行为是不可撤销的，这意味着一旦完成就不可能再变为未完成。
pub struct Completion {
    inner: SpinMutex<CompletionInner>,
}

struct CompletionInner {
    done: bool,
    queue: VecDeque<Arc<Waker>>,
}

impl Completion {
    pub fn new() -> Self {
        Self {
            inner: SpinMutex::new(
                CompletionInner {
                    done: false,
                    queue: VecDeque::new(),
                },
                "completion",
            ),
        }
    }

    /// 等待操作完成
    ///
    /// # Parameters
    ///
    /// - `interruptible`: 是否可以被信号中断，如果为 true 的话则该函数会抛出 [SystemError::EINTR] 错误，
    ///   为 false 的话该函数必然返回 Ok。
    ///
    /// # Panics
    ///
    /// 调用该函数时，不能持有自旋锁，否则会 panic。    
    ///
    /// # Errors
    ///
    /// [SystemError::EINTR] 当 `interruptible` 为 true 时，如果当前睡眠被信号中断，则返回该错误。
    pub fn wait(&self, interruptible: bool) -> Result<(), SystemError> {
        let mut inner = self.inner.lock();
        if inner.done {
            return Ok(());
        }
        let (waiter, waker) = Waiter::new_pair();
        inner.queue.push_back(waker);
        drop(inner);
        if let Err(e) = waiter.wait(interruptible) {
            let inner = self.inner.lock();
            if inner.done { Ok(()) } else { Err(e) }
        } else {
            Ok(())
        }
    }

    /// 完成操作
    pub fn complete(&self) {
        let mut inner = self.inner.lock();
        inner.done = true;
        while let Some(waker) = inner.queue.pop_front() {
            waker.wake();
        }
    }
}
