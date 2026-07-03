use crate::{
    arch::{self, IrqArch},
    driver::cpu::MAX_CPU_COUNT,
    exception::InterruptArch,
    process::{
        context::{ArchTaskContext, TaskContext},
        task::TaskControlBlock,
    },
};
use alloc::sync::Arc;
use core::cell::UnsafeCell;
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
        // SAFETY: 此时中断已经关闭
        &mut cpus[unsafe { arch::cpu::cpu_id() }]
    }
}
