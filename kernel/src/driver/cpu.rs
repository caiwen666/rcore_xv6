use core::sync::atomic::AtomicUsize;

/// 支持的最大 CPU 数量
///
/// TODO 扫描设备树来动态获取
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

/// 在线的 CPU 数量
pub static ONLINE_CPU_COUNT: AtomicUsize = AtomicUsize::new(0);
