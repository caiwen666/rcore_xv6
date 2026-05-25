#![allow(static_mut_refs)]
#![allow(clippy::upper_case_acronyms)]
#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(likely_unlikely)]
#![feature(box_as_ptr)]

use crate::{
    arch::{IrqArch, MMArch},
    driver::{UART0, VIRTIO0, enable_plic, init_plic},
    exception::{InterruptArch, timer::timer_tickets},
    mm::{KERNEL_SPACE, MemoryManagementArch, allocator::kernel::KernelAllocator},
    process::{ProcessManager, cpu::CPUManager, kthread::{exit_kthread, spawn_kthread}, schedule::schedule_loop},
};
use alloc::{boxed::Box, string::String, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};

mod arch;
mod console;
mod driver;
mod exception;
mod lang_items;
mod mm;
mod process;
mod sync;
mod utils;

extern crate alloc;

#[global_allocator]
pub static KERNEL_ALLOCATOR: KernelAllocator = KernelAllocator;

/// 内核的入口函数
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    // SAFETY: 此时中断还没开
    let cpu = unsafe { CPUManager::current_cpu() };
    if cpu.id == 0 {
        println!("{}", include_str!("logo.txt"));
        // 初始化虚拟内存
        MMArch::init();
        // 进入内核内存空间
        KERNEL_SPACE.lock().activate();
        KERNEL_SPACE.lock().print_info(false);
        // 初始化内核进程
        ProcessManager::init();
        // 初始化中断
        IrqArch::init();
        // 初始化 PLIC
        init_plic();
        // 为当前 CPU 启用 PLIC 中断
        enable_plic(cpu.id);
        // 启动第一个内核线程，继续完成后续初始化
        spawn_kthread(kthread_main);
        spawn_kthread(kthread_test);
        STARTED.store(true, Ordering::Release);
    } else {
        while !STARTED.load(Ordering::Acquire) {
            core::hint::spin_loop();
        }
        KERNEL_SPACE.lock().activate();
        IrqArch::init();
        enable_plic(cpu.id);
        println!("CPU {} is ready", cpu.id);
    }
    // SAFETY: 当前尚未开启中断
    unsafe { schedule_loop() };
}

/// 第一个内核线程
pub fn kthread_main() -> ! {
    let task = CPUManager::current_task().expect("kthread_main: current_task is None");
    let mut last: Option<usize> = None;
    println!("hello, world! {}", task.id);
    loop {
        let t = timer_tickets();
        if let Some(prev) = last {
            if t - prev == 100 {
                println!("thread {} timer_tickets = {}", task.id, t);
                last = Some(t);
            }
        } else {
            last = Some(t);
        }
        if t > 300 {
            break;
        }
    }
    println!("VIRTIO0: {} KB", VIRTIO0.capacity() / 1024);
    let mut text_bytes = Vec::with_capacity(2048);
    let mut test_buf = Box::new([0u8; 512]);
    for i in 0..4 {
        VIRTIO0.read_block(i, test_buf.as_mut_slice());
        text_bytes.extend_from_slice(test_buf.as_ref());
    }
    let s = String::from_utf8_lossy(text_bytes.as_slice());
    println!("{}", s);
    exit_kthread();
}

pub fn kthread_test() -> ! {
    let task = CPUManager::current_task().expect("kthread_main: current_task is None");
    let mut last: Option<usize> = None;
    println!("hello, world! {}", task.id);
    loop {
        let t = timer_tickets();
        if let Some(prev) = last {
            if t - prev == 60 {
                println!("thread {} timer_tickets = {}", task.id, t);
                last = Some(t);
            }
        } else {
            last = Some(t);
        }
        if t > 200 {
            break;
        }
    }
    for _ in 0..1000 {
        UART0.put(b'X');
    }
    exit_kthread();
}
