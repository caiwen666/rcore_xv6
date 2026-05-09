pub mod sifive_test;
pub mod uart;
pub mod virtio;

use crate::{
    driver::virtio::{
        device::blk::VirtIOBlk,
        transport::{
            DeviceType, Transport,
            mmio::{MmioTransport, MmioVersion, VirtIOHeader},
        },
    },
    mm::{PhysMemoryArea, PhysMemoryAreaKind, address::PhysAddr},
    sync::spin::SpinMutex,
};
use core::ptr::NonNull;
use lazy_static::lazy_static;
use sifive_test::SiFiveTest;
use uart::Uart;

const SIFIVE_TEST_ADDR: usize = 0x100000;
const UART0_ADDR: usize = 0x10000000;
const VIRTIO0_ADDR: usize = 0x10001000;
const VIRTIO0_MMIO_SIZE: usize = 0x1000;
/// 主内存起始地址和大小
const MAIN_MEMORY_ADDR: usize = 0x80000000;
const MAIN_MEMORY_SIZE: usize = 1024 * 1024 * 128; // 128MB

/// 物理内存区域
pub const MEMORY_AREAS: [PhysMemoryArea; 4] = [
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
    pub static ref UART0: SpinMutex<Uart> = SpinMutex::new(Uart::new(UART0_ADDR), "UART0");
}
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
