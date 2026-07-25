/// 支持的最大 CPU id + 1，所有在线的 CPU id 必须小于这个值，同时这个值也决定了内核中最多可以使用的 CPU 数量
pub const MAX_CPU_COUNT: usize = 8;
/// CPU 的基准时钟周期频率。QEMU 默认为 10MHz，即 1 秒走 10000000 个周期
pub const CLOCK_CYCLE: usize = 10000000;

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

static mut ONLINE_CPU_MASK: usize = 0;

/// 设置在线 CPU
///
/// # Panics
///
/// 如果 CPU 数量大于 [`MAX_CPU_COUNT`]，则 panic
///
/// # Safety
///
/// 必须在 CPU 0 早期初始化时设置，并且只能设置一次
pub unsafe fn set_online_cpu_mask(cpu_mask: usize) {
    let count = cpu_mask.count_ones() as usize;
    assert!(cpu_mask < (1 << MAX_CPU_COUNT), "CPU id is too large");
    assert!(
        count <= MAX_CPU_COUNT,
        "CPU count must be less than or equal to MAX_CPU_COUNT"
    );
    unsafe {
        ONLINE_CPU_MASK = cpu_mask;
    }
}

pub fn online_cpu_mask() -> usize {
    unsafe { ONLINE_CPU_MASK }
}

pub fn online_cpu_count() -> usize {
    unsafe { ONLINE_CPU_MASK.count_ones() as usize }
}
