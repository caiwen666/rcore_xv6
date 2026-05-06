pub mod sifive_test;
pub mod uart;

use crate::{
    mm::{PhysMemoryArea, PhysMemoryAreaKind, address::PhysAddr},
    sync::spin::SpinMutex,
};
use lazy_static::lazy_static;
use sifive_test::SiFiveTest;
use uart::Uart;

const SIFIVE_TEST_ADDR: usize = 0x100000;
const UART0_ADDR: usize = 0x10000000;
const PLIC_ADDR: usize = 0x0c000000;
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
        name: "device_plic",
        base: PhysAddr::new(PLIC_ADDR),
        size: 0x400000,
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
