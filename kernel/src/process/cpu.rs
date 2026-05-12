use crate::{arch, driver::cpu::MAX_CPU_COUNT, sync::spin::SpinState};

#[expect(clippy::upper_case_acronyms)]
pub struct CPU {
    /// 自旋锁的计数器
    pub spinning_state: SpinState,
}

impl CPU {
    pub const fn new() -> Self {
        Self {
            spinning_state: SpinState::new(),
        }
    }
}

pub struct CPUManager {
    cpus: [CPU; MAX_CPU_COUNT],
}

impl CPUManager {
    pub const fn new() -> Self {
        Self {
            cpus: array_macro::array![_ => CPU::new(); MAX_CPU_COUNT],
        }
    }
}

pub static mut CPU_MANAGER: CPUManager = CPUManager::new();

impl CPUManager {
    /// 获取当前 CPU
    pub fn current_cpu(&mut self) -> &mut CPU {
        let cpu_id = arch::cpu::cpu_id();
        &mut self.cpus[cpu_id]
    }
}
