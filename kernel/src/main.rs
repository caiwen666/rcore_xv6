#![allow(static_mut_refs)]
#![allow(clippy::upper_case_acronyms)]
#![no_std]
#![no_main]
#![feature(negative_impls)]
#![feature(likely_unlikely)]
#![feature(never_type)]

use crate::{
    arch::{IrqArch, MMArch},
    driver::{VIRTIO0, cpu::ONLINE_CPU_COUNT, enable_plic, init_plic},
    exception::InterruptArch,
    fs::{
        Ext2FileSystem, ROOT_FS,
        file::FileSeekMethod,
        vfs::{VirtualFileSystem, lookup},
    },
    mm::{KERNEL_SPACE, MemoryManagementArch},
    process::{
        ProcessManager, cpu::CPUManager, kthread::spawn_kthread, schedule::schedule_loop,
        timer::sleep_with_interval,
    },
};
use alloc::string::String;
use alloc::vec;
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
    ONLINE_CPU_COUNT.fetch_add(1, Ordering::Relaxed);
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
pub fn kthread_main() {
    // 打印 LOGO，顺带调用 ROOT_FS 完成根文件系统的初始化
    let logo = lookup(ROOT_FS.root(), "logo.txt").unwrap();
    let mut logo_buf = vec![0u8; logo.metadata().size];
    logo.read_at(0, &mut logo_buf);
    println!("{}", String::from_utf8_lossy(logo_buf.as_slice()));
    // 设置内核进程的工作目录
    let process = ProcessManager::current_resource();
    process.set_cwd(ROOT_FS.root());

    // 初始化块设备
    println!("VIRTIO0: {} KB", VIRTIO0.capacity() / 1024);
    // 挂载文件系统
    let ext2_mountpoint = lookup(ROOT_FS.root(), "root").unwrap();
    ext2_mountpoint.mount(VirtualFileSystem::new(Ext2FileSystem::new(VIRTIO0.clone())));

    // 测试代码
    spawn_kthread(kthread_test);
    let fd = process.open_file("/root/Cargo.lock").unwrap();
    let file = process.get_file(fd).unwrap();
    let file_len = file.seek(FileSeekMethod::End(0)).unwrap();
    file.seek(FileSeekMethod::Absolute(0)).unwrap();
    let mut file_buf = vec![0u8; file_len];
    file.read(&mut file_buf).unwrap();
    println!("{}", String::from_utf8_lossy(file_buf.as_slice()));
}

pub fn kthread_test() {
    let task = CPUManager::current_task().expect("kthread_main: current_task is None");
    println!("hello, world! {}", task.id);
    for i in 0..5 {
        println!("thread {} countdown: {}", task.id, 5 - i);
        sleep_with_interval(Duration::from_secs(1));
    }
    for i in 0..10 {
        spawn_kthread(move || {
            println!("hello, world! {}", i);
        });
    }

    let process = ProcessManager::current_resource();
    let fd = process.open_file("stdin").unwrap();
    let stdin = process.get_file(fd).unwrap();
    let fd = process.open_file("stdout").unwrap();
    let stdout = process.get_file(fd).unwrap();
    let mut buf = [0u8; 4];
    loop {
        stdin.read(&mut buf).unwrap();
        stdout.write(&buf).unwrap();
    }
}
