use crate::{arch, process::cpu::CPU_MANAGER};
use core::{
    cell::{Cell, UnsafeCell},
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// 挂在 CPU 上的，表示当前 CPU 上面的自旋锁的状态。
pub struct SpinState {
    /// 自旋锁的数量
    pub count: usize,
    /// 在加自旋锁之前，是否关闭了中断
    pub interrupted: bool,
}

impl SpinState {
    pub const fn new() -> Self {
        Self {
            count: 0,
            interrupted: false,
        }
    }

    /// 在当前 CPU 上加自旋锁
    ///
    /// 如果是第一次加锁，会关闭中断
    pub fn push_lock(&mut self) {
        let old_interrupted = arch::interrupt::get_interrupt_state();
        arch::interrupt::disable_interrupt();
        // 一定是先关中断，再对 SpinningState 进行操作，否则中间可能会有中断进来导致出现竞态
        if self.count == 0 {
            self.interrupted = old_interrupted;
        }
        self.count += 1;
    }

    /// 在当前 CPU 上移除自旋锁
    ///
    /// 如果是最后一次移除自旋锁，会把中断状态恢复到加第一个锁之前的状态
    ///
    /// # Panics
    ///
    /// - 当前不能开启中断，否则会 panic
    /// - 当前 CPU 上面必须已经施加过自旋锁，否则会 panic
    pub fn pop_lock(&mut self) {
        assert!(
            !arch::interrupt::get_interrupt_state(),
            "release a lock that is not acquired"
        );
        assert!(self.count > 0, "release a lock that is not acquired");
        self.count -= 1;
        if self.count == 0 && self.interrupted {
            arch::interrupt::enable_interrupt();
        }
    }
}

pub struct SpinMutex<T: ?Sized> {
    lock: AtomicBool,
    // 当前锁被哪个 CPU 持有，用于死锁检测
    cpu_id: Cell<Option<usize>>,
    // 锁的名称，用于死锁检测时显示
    name: &'static str,
    inner: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}

pub struct SpinMutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a SpinMutex<T>,
}

impl<T: ?Sized> !Send for SpinMutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinMutexGuard<'_, T> {}

impl<T> SpinMutex<T> {
    pub const fn new(inner: T, name: &'static str) -> Self {
        Self {
            lock: AtomicBool::new(false),
            inner: UnsafeCell::new(inner),
            cpu_id: Cell::new(None),
            name,
        }
    }
}

impl<T: ?Sized> SpinMutex<T> {
    pub fn try_lock_weak(&self) -> Option<SpinMutexGuard<'_, T>> {
        if self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(SpinMutexGuard { lock: self })
        } else {
            None
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        unsafe {
            let cpu = CPU_MANAGER.current_cpu();
            cpu.spinning_state.push_lock()
        };
        if core::hint::unlikely(self.check_holding()) {
            panic!("deadlock detected: {} is locked by the same CPU", self.name);
        }
        loop {
            if let Some(guard) = self.try_lock_weak() {
                self.cpu_id.set(Some(arch::cpu::cpu_id()));
                break guard;
            }

            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
    }

    pub fn unlock(&self) {
        if core::hint::unlikely(!self.check_holding()) {
            panic!("unlock a lock that is not held: {}", self.name);
        }
        self.cpu_id.set(None);
        self.lock.store(false, Ordering::Release);
        unsafe { CPU_MANAGER.current_cpu().spinning_state.pop_lock() };
    }

    /// 检查当前锁是否被当前 CPU 持有
    fn check_holding(&self) -> bool {
        self.cpu_id
            .get()
            .is_some_and(|cpu_id| cpu_id == arch::cpu::cpu_id())
    }
}

impl<T: ?Sized> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.inner.get() }
    }
}

impl<T: ?Sized> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.inner.get() }
    }
}

impl<T: ?Sized> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SpinMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for SpinMutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
