use core::time::Duration;

use super::RiscV64InterruptArch;
use crate::{
    arch::register::scause::{Exception, Interrupt},
    driver::plic_handler,
    exception::{InterruptArch, timer::timer_handler},
    println,
    process::{cpu::CPUManager, timer::sleep_with_interval},
};
use riscv::register::{scause, sepc, sip, sstatus, stval, stvec};

core::arch::global_asm!(include_str!("utils.S"));
core::arch::global_asm!(include_str!("trap_from_kernel.S"));
core::arch::global_asm!(include_str!("trap_from_user.S"));

/// 从内核态陷入到内核态时的处理
#[unsafe(no_mangle)]
unsafe extern "C" fn kernel_trap_handler() {
    let reg_sstatus = sstatus::read();
    let reg_sepc = sepc::read();
    let reg_stval = stval::read();
    let reg_scause = scause::read();

    assert!(
        reg_sstatus.spp() == sstatus::SPP::Supervisor,
        "kernel_trap_handler: not from supervisor mode"
    );
    assert!(
        !RiscV64InterruptArch::get_interrupt_state(),
        "kernel_trap_handler: interrupts should not be enabled"
    );

    match reg_scause.cause() {
        scause::Trap::Interrupt(code) => match Interrupt::from(code) {
            Interrupt::SupervisorExternal => {
                // 来自 PLIC 的中断
                unsafe { plic_handler() };
            }
            Interrupt::SupervisorSoft => {
                // M 模式将时钟中断以软件中断的方式转发到 S 模式
                // 清除 S 模式软件中断，表示处理完毕，一定要在 timer_handler 之前清除，
                // 因为 timer_handler 可能会挂起当前线程
                unsafe { sip::clear_ssoft() };
                // SAFETY: 当前函数已经保证了中断关闭
                unsafe { timer_handler(true) };
            }
            interrupt => {
                println!(
                    "scause:\t{:?}\nsepc:\t0x{:x}\nstval:\t0x{:x}",
                    interrupt, reg_sepc, reg_stval
                );
                panic!("Unresolved Interrupt.")
            }
        },
        scause::Trap::Exception(code) => {
            let exception = Exception::from(code);
            println!(
                "scause:\t{:?}\nsepc:\t0x{:x}\nstval:\t0x{:x}",
                exception, reg_sepc, reg_stval
            );
            panic!("Unresolved Exception.")
        }
    }

    // 时钟中断可能会导致其他的线程调度到当前 CPU，这可能出现一些状态寄存器的变化，所以需要还原
    unsafe {
        sstatus::write(reg_sstatus);
        sepc::write(reg_sepc)
    };
}

/// 从用户态陷入到内核态时的处理
#[unsafe(no_mangle)]
unsafe extern "C" fn user_trap_handler() {
    let reg_sstatus = sstatus::read();
    assert!(
        reg_sstatus.spp() == sstatus::SPP::User,
        "user_trap_handler: not from user mode"
    );

    unsafe extern "C" {
        fn trap_from_kernel();
    }
    let mut reg_stvec = stvec::read();
    reg_stvec.set_trap_mode(stvec::TrapMode::Direct);
    reg_stvec.set_address(trap_from_kernel as *const () as usize);
    unsafe { stvec::write(reg_stvec) };

    let reg_scause = scause::read();
    let current_task = CPUManager::current_task().unwrap();
    match reg_scause.cause() {
        scause::Trap::Exception(code) => match Exception::from(code) {
            // 系统调用
            Exception::UserEnvCall => {
                let trap_context = current_task.trap_context();
                trap_context.sepc += 4;

                let syscall_id = trap_context.x[17];
                // 系统调用可能耗时较长，这里开中断
                RiscV64InterruptArch::enable_interrupt();

                // 测试代码
                println!(
                    "[syscall begin] pid: {}, tid: {}, syscall_id: {}",
                    current_task.process().pid,
                    current_task.id,
                    syscall_id
                );
                sleep_with_interval(Duration::from_secs(1));
                trap_context.x[10] = syscall_id + 1;
                println!("[syscall end] result: {}", trap_context.x[10]);
            }
            exception => {
                let reg_sepc = sepc::read();
                let reg_stval = stval::read();
                println!(
                    "pid:{}, tid:{}\nscause:\t{:?}\nsepc:\t0x{:x}\nstval:\t0x{:x}",
                    current_task.process().pid,
                    current_task.id,
                    exception,
                    reg_sepc,
                    reg_stval
                );
                panic!("Unresolved Exception.")
            }
        },
        scause::Trap::Interrupt(code) => match Interrupt::from(code) {
            // 前两个和 kernel_trap_handler 中的处理一致
            Interrupt::SupervisorExternal => {
                unsafe { plic_handler() };
            }
            Interrupt::SupervisorSoft => {
                unsafe { sip::clear_ssoft() };
                // SAFETY: 当前函数已经保证了中断关闭
                unsafe { timer_handler(false) };
            }
            interrupt => {
                let reg_sepc = sepc::read();
                let reg_stval = stval::read();
                println!(
                    "pid:{}, tid:{}\nscause:\t{:?}\nsepc:\t0x{:x}\nstval:\t0x{:x}",
                    current_task.process().pid,
                    current_task.id,
                    interrupt,
                    reg_sepc,
                    reg_stval
                );
                panic!("Unresolved Interrupt.")
            }
        },
    }

    drop(current_task);
    RiscV64InterruptArch::return_to_user();
}
