use super::RiscV64InterruptArch;
use crate::{
    arch::register::scause::{Exception, Interrupt}, driver::plic_handler, exception::{InterruptArch, timer::timer_handler}, println
};
use riscv::register::{scause, sepc, sip, sstatus, stval};

core::arch::global_asm!(include_str!("trap_from_kernel.S"));

#[unsafe(no_mangle)]
/// 从内核态陷入到内核态时的处理
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
