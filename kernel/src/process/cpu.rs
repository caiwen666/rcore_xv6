use crate::{arch, config, sync::spin::SpinState};

// riscv 要求栈对齐到 16 字节
#[repr(align(16))]
#[expect(dead_code)]
struct AlignedBootStack([u8; 4096 * config::MAX_CPU_COUNT]);
/// CPU 初始化时的栈
///
/// 栈大小固定为 4096 字节，如果发生了修改，还需要同步修改 `arch/entry.S`
#[unsafe(no_mangle)]
static BOOT_STACK: AlignedBootStack = AlignedBootStack([0; 4096 * config::MAX_CPU_COUNT]);

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
    cpus: [CPU; config::MAX_CPU_COUNT],
}

impl CPUManager {
    pub const fn new() -> Self {
        Self {
            cpus: array_macro::array![_ => CPU::new(); config::MAX_CPU_COUNT],
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
