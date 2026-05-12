use crate::{arch, driver::cpu::MAX_CPU_COUNT, sync::spin::SpinState};
use core::cell::UnsafeCell;
use lazy_static::lazy_static;

#[expect(clippy::upper_case_acronyms)]
pub struct CPU {
    /// 自旋锁的计数器
    pub spinning_state: SpinState,
    pub id: usize,
}

impl CPU {
    pub(self) fn new(id: usize) -> Self {
        Self {
            spinning_state: SpinState::new(),
            id,
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
        &mut cpus[arch::cpu::cpu_id()]
    }
}
