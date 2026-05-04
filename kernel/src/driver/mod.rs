pub mod sifive_test;
mod uart;

use crate::sync::spin::SpinMutex;
use lazy_static::lazy_static;
use sifive_test::SiFiveTest;
use uart::Uart;

const UART0_ADDR: usize = 0x10000000;
lazy_static! {
    pub static ref UART0: SpinMutex<Uart> = SpinMutex::new(Uart::new(UART0_ADDR), "UART0");
}

const SIFIVE_TEST_ADDR: usize = 0x100000;
pub static SIFIVE_TEST: SiFiveTest = SiFiveTest::new(SIFIVE_TEST_ADDR);
