#![allow(static_mut_refs)]
#![allow(clippy::upper_case_acronyms)]
#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(likely_unlikely)]

use crate::{
    arch::MMArch,
    driver::VIRTIO0,
    mm::{KERNEL_SPACE, MemoryManagementArch, allocator::kernel::KernelAllocator},
};
use alloc::string::String;

mod arch;
mod console;
mod driver;
mod exception;
mod lang_items;
mod mm;
mod process;
mod sync;

extern crate alloc;

#[global_allocator]
pub static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;

/// 内核的入口函数
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    if arch::cpu::cpu_id() == 0 {
        println!("{}", include_str!("logo.txt"));
        // 初始化虚拟内存
        MMArch::init();
        KERNEL_SPACE.lock().print_info(false);
        println!("VIRTIO0: {} KB", VIRTIO0.lock().capacity() / 1024);
        let mut test_buf = [0u8; 2048];
        for i in 0..4 {
            VIRTIO0
                .lock()
                .read_block_sync(i, &mut test_buf[i * 512..(i + 1) * 512]);
        }
        let s = String::from_utf8_lossy(&test_buf);
        println!("{}", s);
        driver::SIFIVE_TEST.shutdown(driver::sifive_test::ShutdownReason::Normal, 0);
    }
    loop {
        core::hint::spin_loop();
    }
}
