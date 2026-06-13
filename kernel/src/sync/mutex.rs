use crate::{
    process::cpu::CPUManager,
    sync::{condvar::Condvar, spin::SpinMutex},
};
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
};

pub struct Mutex<T: ?Sized> {
    /// 锁的名称，用于死锁检测时显示
    name: &'static str,
    inner: SpinMutex<MutexInner>,
    condvar: Condvar,
    data: UnsafeCell<T>,
}

pub struct MutexInner {
    /// 当前锁的持有者，Option 为 None 则为没人持有，为 Some 时
    /// 元组中第一个数字表示 pid，第二个数字表示 tid
    owner: Option<(usize, usize)>,
    /// 当前锁是否被占有
    locked: bool,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
}

impl<T: ?Sized> !Send for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T> Mutex<T> {
    pub fn new(inner: T, name: &'static str) -> Self {
        Self {
            name,
            inner: SpinMutex::new(
                MutexInner {
                    owner: None,
                    locked: false,
                },
                name,
            ),
            data: UnsafeCell::new(inner),
            condvar: Condvar::new(),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        let mut inner = self.inner.lock();
        while inner.locked {
            let (owner_pid, owner_tid) = inner.owner.expect("Mutex is locked but no owner");
            let current_task = CPUManager::current_task().expect("Cannot get current task");
            if current_task.id == owner_tid && current_task.process().pid == owner_pid {
                panic!(
                    "deadlock detected: {} is locked by the same task, pid: {}, tid: {}",
                    self.name, owner_pid, owner_tid
                );
            }
            inner = self.condvar.wait(inner);
        }
        inner.locked = true;
        let current_task = CPUManager::current_task().expect("Cannot get current task");
        inner.owner = Some((current_task.process().pid, current_task.id));
        MutexGuard { mutex: self }
    }

    fn unlock(&self) {
        let mut inner = self.inner.lock();
        let current_task = CPUManager::current_task().expect("Cannot get current task");
        let (owner_pid, owner_tid) = inner.owner.expect("Mutex is locked but no owner");
        if owner_pid != current_task.process().pid || owner_tid != current_task.id {
            panic!("Mutex is not locked by the current task");
        }
        inner.owner = None;
        inner.locked = false;
        self.condvar.notify_all();
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
