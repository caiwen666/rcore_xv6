use super::config::MAX_CPU_COUNT;
use crate::{arch, sync::spin::SpinState};

/// 每个核的 M 态引导栈大小（须与 `arch/entry.S` 里 `li a0, ...` 一致）
pub const BOOT_STACK_SIZE: usize = 1024 * 1024;

// riscv 要求栈对齐到 16 字节
#[repr(align(16))]
#[expect(dead_code)]
struct AlignedBootStack([u8; BOOT_STACK_SIZE * MAX_CPU_COUNT]);
/// CPU 初始化时的栈
#[unsafe(no_mangle)]
#[unsafe(link_section = ".data.boot_stack")]
static BOOT_STACK: AlignedBootStack = AlignedBootStack([0; BOOT_STACK_SIZE * MAX_CPU_COUNT]);

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
