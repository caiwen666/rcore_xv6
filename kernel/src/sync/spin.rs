use crate::{arch::IrqArch, exception::InterruptArch, process::cpu::CPUManager};
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

pub struct SpinMutex<T: ?Sized> {
    lock: AtomicBool,
    // 当前锁被哪个 CPU 持有，用于死锁检测，usize::MAX 表示没有 CPU 持有
    owner_hart: AtomicUsize,
    // 锁的名称，用于死锁检测时显示
    name: &'static str,
    inner: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}

pub struct SpinMutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a SpinMutex<T>,
    leaked: bool,
}

impl<T: ?Sized> !Send for SpinMutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinMutexGuard<'_, T> {}

impl<T> SpinMutex<T> {
    pub const fn new(inner: T, name: &'static str) -> Self {
        Self {
            lock: AtomicBool::new(false),
            inner: UnsafeCell::new(inner),
            owner_hart: AtomicUsize::new(usize::MAX),
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
            Some(SpinMutexGuard {
                lock: self,
                leaked: false,
            })
        } else {
            None
        }
    }

    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        let old_state = IrqArch::get_interrupt_state();
        IrqArch::disable_interrupt();
        // SAFETY: 已经关闭了中断
        let cpu = unsafe { CPUManager::current_cpu() };
        cpu.spinning_state.push_lock(old_state);

        if core::hint::unlikely(unsafe { self.check_holding() }) {
            panic!("deadlock detected: {} is locked by the same CPU", self.name);
        }
        loop {
            if let Some(guard) = self.try_lock_weak() {
                self.owner_hart.store(cpu.id, Ordering::Relaxed);
                break guard;
            }

            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
    }

    /// # Safety
    ///
    /// 调用者需要确保自己当前是关中断的，并且拥有 SpinMutexGuard
    pub unsafe fn unlock(&self) {
        if core::hint::unlikely(!unsafe { self.check_holding() }) {
            panic!("unlock a lock that is not held: {}", self.name);
        }
        self.owner_hart.store(usize::MAX, Ordering::Relaxed);
        self.lock.store(false, Ordering::Release);
        // SAFETY: 此时还处于中断关闭期中
        let cpu = unsafe { CPUManager::current_cpu() };
        if cpu.spinning_state.pop_lock() {
            IrqArch::enable_interrupt();
        }
    }

    /// 检查当前锁是否被当前 CPU 持有
    ///
    /// # Safety
    ///
    /// 调用时需要保证中断关闭
    unsafe fn check_holding(&self) -> bool {
        // SAFETY: 调用者保证了此时中断关闭
        let cpu = unsafe { CPUManager::current_cpu() };
        self.owner_hart.load(Ordering::Relaxed) == cpu.id
    }
}

impl<T: ?Sized> SpinMutexGuard<'_, T> {
    /// 将 SpinMutexGuard 泄露，使其在 drop 时不会被自动解锁
    ///
    /// # Safety
    ///
    /// 调用者需要确保在调用该函数之后，自己手动释放锁
    pub unsafe fn leak(&mut self) {
        self.leaked = true;
    }
}

impl<T: ?Sized> SpinMutexGuard<'_, T> {
    /// 获取 SpinMutexGuard 对应的 SpinMutex
    ///
    /// 主要用于 Condvar
    pub(super) fn lock(&self) -> &SpinMutex<T> {
        self.lock
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
        if !self.leaked {
            unsafe { self.lock.unlock() }
        }
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
