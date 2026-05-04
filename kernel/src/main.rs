#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(likely_unlikely)]
#![allow(static_mut_refs)]

mod arch;
mod config;
mod console;
mod driver;
mod lang_items;
mod process;
mod sync;

/// 内核的入口函数
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    if arch::cpu::cpu_id() == 0 {
        println!("hello, world! from cpu {}", arch::cpu::cpu_id());
        driver::SIFIVE_TEST.shutdown(driver::sifive_test::ShutdownReason::Normal, 0);
    }
    loop {}
}
