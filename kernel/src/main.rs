#![allow(static_mut_refs)]
#![allow(clippy::upper_case_acronyms)]
#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(likely_unlikely)]
#![feature(box_as_ptr)]
#![feature(never_type)]

use crate::{
    arch::{IrqArch, MMArch},
    driver::{UART0, VIRTIO0, enable_plic, init_plic},
    exception::InterruptArch,
    fs::{ROOT_FS, vfs::lookup},
    mm::{KERNEL_SPACE, MemoryManagementArch},
    process::{
        ProcessManager,
        cpu::CPUManager,
        kthread::{exit_kthread, spawn_kthread},
        schedule::schedule_loop,
        timer::sleep_with_interval,
    },
};
use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

mod arch;
mod console;
mod driver;
mod exception;
mod fs;
mod lang_items;
mod mm;
mod process;
mod sync;
mod utils;

extern crate alloc;

/// 内核的入口函数
#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() {
    static STARTED: AtomicBool = AtomicBool::new(false);
    // SAFETY: 此时中断还没开
    let cpu = unsafe { CPUManager::current_cpu() };
    if cpu.id == 0 {
        println!("rcore_xv6 kernel is booting");
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
    // 打印 LOGO，顺带调用 ROOT_FS 完成根文件系统的初始化
    println!("{:?}", ROOT_FS.root().list());
    let logo = lookup(ROOT_FS.root(), "logo.txt");
    if let Some(logo) = logo {
        let mut buf = vec![0u8; logo.metadata().size];
        logo.read_at(0, &mut buf);
        let s = String::from_utf8_lossy(buf.as_slice());
        println!("{}", s);
    } else {
        println!("logo.txt not found");
    }
    spawn_kthread(kthread_test);
    let task = CPUManager::current_task().expect("kthread_main: current_task is None");
    println!("hello, world! {}", task.id);
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
    println!("hello, world! {}", task.id);
    for i in 0..5 {
        println!("thread {} countdown: {}", task.id, 5 - i);
        sleep_with_interval(Duration::from_secs(1));
    }
    for i in 0..10 {
        spawn_kthread(move || {
            println!("hello, world! {}", i);
            exit_kthread();
        });
    }
    for _ in 0..1000 {
        UART0.put(b'X');
    }
    exit_kthread();
}
