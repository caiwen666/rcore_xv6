use core::cell::UnsafeCell;

use crate::{
    arch::{self, IrqArch},
    driver::cpu::MAX_CPU_COUNT,
    exception::InterruptArch,
    process::{
        context::{ArchTaskContext, TaskContext},
        schedule::TaskScheduler,
        task::{TaskControlBlock, TaskStatus},
    },
};
use alloc::sync::Arc;
use lazy_static::lazy_static;

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

    pub fn push_lock(&mut self, old_state: bool) {
        if self.count == 0 {
            self.interrupted = old_state;
        }
        self.count += 1;
    }

    /// # Returns
    ///
    /// 返回一个 bool，表示是否需要开启中断
    ///
    /// # Panics
    ///
    /// - 当前不能开启中断，否则会 panic
    /// - 当前 CPU 上面必须已经施加过自旋锁，否则会 panic
    pub fn pop_lock(&mut self) -> bool {
        assert!(
            !IrqArch::get_interrupt_state(),
            "release a lock that is not acquired"
        );
        assert!(self.count > 0, "release a lock that is not acquired");
        self.count -= 1;
        self.count == 0 && self.interrupted
    }
}

#[expect(clippy::upper_case_acronyms)]
pub struct CPU {
    /// 自旋锁的计数器
    pub spinning_state: SpinState,
    pub id: usize,
    pub current_task: Option<Arc<TaskControlBlock>>,
    /// 位于调度循环的上下文
    pub idle_task_context: ArchTaskContext,
}

impl CPU {
    pub(self) fn new(id: usize) -> Self {
        Self {
            spinning_state: SpinState::new(),
            id,
            current_task: None,
            idle_task_context: ArchTaskContext::zero_init(),
        }
    }

    /// 将当前任务调度出去
    pub fn yield_current_task(&mut self) {
        // 由于获取当前 CPU 需要关闭中断，所以此时中断就是关闭的
        let Some(current_task) = self.current_task.clone() else {
            return;
        };
        let mut task_inner = current_task.lock();
        // TODO 有必要判这个吗？
        if task_inner.status != TaskStatus::Running {
            return;
        }
        task_inner.status = TaskStatus::Ready;
        TaskScheduler::push(current_task.clone());
        let current_context = &mut task_inner.context as *mut _;
        // 有可能回到调度器之后，当前任务被杀死，然后永远回不来了
        // 所以这里需要把当前函数作用域内的 Arc 释放，否则这里永远贡献一个 Arc 引用计数，使得当前任务永远无法被回收
        // 由于当前的 task_inner 引用自 current_task，所以需要先把 task_inner 搞掉，但是同时又不能释放锁
        // SAFETY: 去往调度循环之后，调度循环那边会释放锁
        unsafe { task_inner.leak() };
        drop(task_inner);
        drop(current_task);
        // 当前 CPU 的 current_task 还保留一个引用计数，所以这里的 current_task 不会变为悬垂引用
        self.go_scheduler(current_context);
        // 这里需要重新获取当前 CPU 的引用，因为当前任务可能已经被调度到别的 CPU 上
        let cpu = unsafe { CPUManager::current_cpu() };
        let current_task = cpu.current_task.clone().unwrap();
        // SAFETY: 到这里说明从调度循环回来了。在回来之前，调度循环会加锁
        unsafe { current_task.unlock() };
    }

    /// 将当前上下文保存，并回到调度循环中
    ///
    /// 整个过程中必须对任务加着锁
    ///
    /// # Notes
    ///
    /// 注意，该函数返回之后，需要重新获取当前 CPU 的引用，不能再用调用该函数之前的 CPU 引用了，
    /// 因为当前任务可能已经被调度到别的 CPU 上
    pub fn go_scheduler(&mut self, current_context: *mut ArchTaskContext) {
        // 由于获取当前 CPU 需要关闭中断，所以此时中断就是关闭的
        // 这里应该确保只有对 task_inner 的锁，不能再有其他的自旋锁
        // 否则多余的自旋锁会直到当前任务被调度回来才能被释放
        if core::hint::unlikely(self.spinning_state.count != 1) {
            panic!("to_scheduler: spinning_state.count != 1");
        }
        let interrupted = self.spinning_state.interrupted;
        // 这里会回到调度循环中，但是调度循环那边，在调度该任务的时候会持有一个锁
        // 这里的锁会在调度循环那里被释放掉
        IrqArch::switch_context(current_context, &mut self.idle_task_context as *mut _);
        self.spinning_state.interrupted = interrupted;
    }
}

pub struct CPUManager {
    cpus: UnsafeCell<[CPU; MAX_CPU_COUNT]>,
}

// SAFETY: 不同 CPU 只会访问自己对应的元素。同一个 CPU 中，访问元素的时候是关中断的
unsafe impl Sync for CPUManager {}

lazy_static! {
    static ref CPU_MANAGER: CPUManager = CPUManager::new();
}

impl CPUManager {
    pub(self) fn new() -> Self {
        Self {
            cpus: UnsafeCell::new(array_macro::array![id => CPU::new(id); MAX_CPU_COUNT]),
        }
    }

    /// 获取当前 CPU
    ///
    /// # Safety
    ///
    /// 调用时需要保证中断关闭
    pub unsafe fn current_cpu() -> &'static mut CPU {
        let cpus = unsafe { &mut *CPU_MANAGER.cpus.get() };
        &mut cpus[arch::cpu::cpu_id()]
    }

    /// 获取当前任务
    pub fn current_task() -> Option<Arc<TaskControlBlock>> {
        let interrupted = IrqArch::get_interrupt_state();
        IrqArch::disable_interrupt();
        let cpu = unsafe { CPUManager::current_cpu() };
        cpu.spinning_state.push_lock(interrupted);
        let task = cpu.current_task.clone();
        if cpu.spinning_state.pop_lock() {
            IrqArch::enable_interrupt();
        }
        task
    }
}
