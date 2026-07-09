use super::RiscV64InterruptArch;
use crate::{
    arch::register::scause::{Exception, Interrupt},
    driver::plic_handler,
    exception::{InterruptArch, syscall::syscall_table, timer::timer_handler},
    println,
    process::ProcessManager,
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
    let current_task = ProcessManager::current_task();
    match reg_scause.cause() {
        scause::Trap::Exception(code) => match Exception::from(code) {
            // 系统调用
            Exception::UserEnvCall => {
                let (syscall_id, args) = unsafe {
                    current_task.with_trap_context(move |trap_context| {
                        // a7 存放 syscall id
                        let syscall_id = trap_context.x[17];
                        // a0-a5 存放参数
                        let args = [
                            trap_context.x[10],
                            trap_context.x[11],
                            trap_context.x[12],
                            trap_context.x[13],
                            trap_context.x[14],
                            trap_context.x[15],
                        ];
                        trap_context.sepc += 4;
                        (syscall_id, args)
                    })
                };

                if let Some(handle) = syscall_table().get(syscall_id) {
                    // 系统调用可能耗时较长，这里开中断
                    RiscV64InterruptArch::enable_interrupt();
                    match (handle.handle)(args) {
                        Ok(result) => {
                            if result > isize::MAX as usize {
                                panic!(
                                    "Syscall result too large, syscall id: {}, result: {}",
                                    syscall_id, result
                                );
                            }
                            unsafe {
                                current_task.with_trap_context(move |trap_context| {
                                    trap_context.x[10] = result;
                                });
                            }
                        }
                        Err(error) => unsafe {
                            current_task.with_trap_context(move |trap_context| {
                                trap_context.x[10] = error.posix_errno() as usize;
                            });
                        },
                    }
                } else {
                    println!(
                        "Unresolved Syscall. The thread has been killed.\nprocess_id: {}\nthread_id: {}\nsyscall_id: {}",
                        current_task.process().pid,
                        current_task.id,
                        syscall_id
                    );
                    current_task.lock().killed = true;
                }
            }
            exception => {
                let reg_sepc = sepc::read();
                let reg_stval = stval::read();
                println!(
                    "Unresolved Exception. The thread has been killed.\nprocess_id: {}\nthread_id: {}\nscause: {:?}\nsepc: 0x{:x}\nstval: 0x{:x}",
                    current_task.process().pid,
                    current_task.id,
                    exception,
                    reg_sepc,
                    reg_stval
                );
                current_task.lock().killed = true;
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
                    "Unresolved Interrupt. The thread has been killed.\tprocess_id: {}\nthread_id: {}\nscause: {:?}\nsepc: 0x{:x}\nstval: 0x{:x}",
                    current_task.process().pid,
                    current_task.id,
                    interrupt,
                    reg_sepc,
                    reg_stval
                );
                current_task.lock().killed = true;
            }
        },
    }

    // 判断当前线程是否被杀死了，是的话就退出线程
    let is_killed = current_task.lock().killed;
    // 无论是否被杀死，都 drop 掉，因为后面都不会返回了
    drop(current_task);

    if is_killed {
        ProcessManager::exit()
    } else {
        RiscV64InterruptArch::return_to_user();
    }
}
