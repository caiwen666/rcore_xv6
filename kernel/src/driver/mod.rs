pub mod cpu;
pub mod sifive_test;
pub mod uart;
pub mod virtio;
pub mod plic;

use crate::{
    driver::{plic::PLIC, virtio::{
        device::blk::VirtIOBlk,
        transport::{
            DeviceType, Transport,
            mmio::{MmioTransport, MmioVersion, VirtIOHeader},
        },
    }}, mm::{PhysMemoryArea, PhysMemoryAreaKind, address::PhysAddr}, process::cpu::CPUManager, sync::spin::SpinMutex
};
use core::ptr::NonNull;
use lazy_static::lazy_static;
use sifive_test::SiFiveTest;
use uart::Uart;

const SIFIVE_TEST_ADDR: usize = 0x100000;
const UART0_ADDR: usize = 0x10000000;
const PLIC_ADDR: usize = 0x0c000000;
pub const CLINT_ADDR: usize = 0x2000000;
const VIRTIO0_ADDR: usize = 0x10001000;
const VIRTIO0_MMIO_SIZE: usize = 0x1000;
/// 主内存起始地址和大小
const MAIN_MEMORY_ADDR: usize = 0x80000000;
const MAIN_MEMORY_SIZE: usize = 1024 * 1024 * 128; // 128MB

/// 物理内存区域
pub const MEMORY_AREAS: [PhysMemoryArea; 6] = [
    PhysMemoryArea {
        name: "device_sifive_test",
        base: PhysAddr::new(SIFIVE_TEST_ADDR),
        size: 0x1000,
        kind: PhysMemoryAreaKind::Device,
    },
    PhysMemoryArea {
        name: "device_uart0",
        base: PhysAddr::new(UART0_ADDR),
        size: 0x1000,
        kind: PhysMemoryAreaKind::Device,
    },
    PhysMemoryArea {
        name: "device_plic",
        base: PhysAddr::new(PLIC_ADDR),
        size: 0x400000,
        kind: PhysMemoryAreaKind::Device,
    },
    PhysMemoryArea {
        name: "device_clint",
        base: PhysAddr::new(CLINT_ADDR),
        size: 0x10000,
        kind: PhysMemoryAreaKind::Device,
    },
    PhysMemoryArea {
        name: "device_virtio0",
        base: PhysAddr::new(VIRTIO0_ADDR),
        size: VIRTIO0_MMIO_SIZE,
        kind: PhysMemoryAreaKind::Device,
    },
    PhysMemoryArea {
        name: "main_memory",
        base: PhysAddr::new(MAIN_MEMORY_ADDR),
        size: MAIN_MEMORY_SIZE,
        kind: PhysMemoryAreaKind::MainMemory,
    },
];

lazy_static! {
    pub static ref UART0: Uart = Uart::new(UART0_ADDR);
}
pub static PLIC_INSTANCE: PLIC = PLIC::new(PLIC_ADDR);
pub static SIFIVE_TEST: SiFiveTest = SiFiveTest::new(SIFIVE_TEST_ADDR);
lazy_static! {
    pub static ref VIRTIO0: SpinMutex<VirtIOBlk<MmioTransport<'static>>> = {
        // SAFETY: 我们只对 VIRTIO0 建立一次 Transport 实例，并且 VIRTIO0 一直存在。
        let transport = unsafe {
            MmioTransport::new(
                NonNull::new(VIRTIO0_ADDR as *mut VirtIOHeader).unwrap(),
                VIRTIO0_MMIO_SIZE,
            )
        };
        if transport.device_type() != DeviceType::Block {
            panic!("VIRTIO0: Not a block device");
        }
        if transport.version() != MmioVersion::Legacy {
            panic!("VIRTIO0: Unsupported MMIO version: {:?}", transport.version());
        }
        if transport.vendor_id() != 0x554d4551 {
            panic!("VIRTIO0: Unsupported vendor ID: 0x{:X}", transport.vendor_id());
        }
        let virtio_blk = VirtIOBlk::new(transport);
        SpinMutex::new(virtio_blk, "VIRTIO0")
    };
}

pub const UART0_IRQ: u32 = 10;

/// 初始化 PLIC
pub fn init_plic() {
    PLIC_INSTANCE.set_priority(UART0_IRQ, 1);
}

/// 为指定 CPU 启用 PLIC 中断
pub fn enable_plic(cpu_id: usize) {
    PLIC_INSTANCE.set_supervisor_enable(cpu_id, 1 << UART0_IRQ);
    PLIC_INSTANCE.set_supervisor_threshold(cpu_id, 0);
}

/// # SAFETY
/// 
/// 调用该函数的时候需要保证中断关闭
pub unsafe fn plic_handler() {
    let cpu = unsafe { CPUManager::current_cpu() };
    let irq = PLIC_INSTANCE.get_current_interrupt(cpu.id);
    // 外部硬件产生中断后，PLIC 会给所有的 CPU 核心触发外部中断
    // 但是只有一个 CPU 能去成功认领这个外部硬件的中断
    // 没有抢到的 CPU 的 irq 会读到 0
    if irq == 0 {
        return;
    }
    match irq {
        UART0_IRQ => {
            UART0.handle_interrupt();
        }
        _ => {
            panic!("Unknown PLIC interrupt: {}", irq);
        }
    }
    PLIC_INSTANCE.complete_interrupt(cpu.id, irq);
}