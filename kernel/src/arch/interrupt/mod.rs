mod context;
mod handler;

use crate::{
    driver::{
        CLINT_ADDR,
        cpu::{CLOCK_CYCLE, MAX_CPU_COUNT},
    },
    exception::{InterruptArch, timer::TIMER_INTERVAL},
};
use riscv::register::{mie, mscratch, mstatus, mtvec, sstatus, stvec};

core::arch::global_asm!(include_str!("timer.S"));

pub struct RiscV64InterruptArch;

impl InterruptArch for RiscV64InterruptArch {
    type TaskContext = context::TaskContext;

    #[inline]
    fn enable_interrupt() {
        let mut status = sstatus::read();
        status.set_sie(true);
        unsafe { sstatus::write(status) };
    }

    #[inline]
    fn disable_interrupt() {
        let mut status = sstatus::read();
        status.set_sie(false);
        unsafe { sstatus::write(status) };
    }

    #[inline]
    fn get_interrupt_state() -> bool {
        let status = sstatus::read();
        status.sie()
    }

    #[inline]
    fn init() {
        unsafe extern "C" {
            fn trap_from_kernel();
        }
        let mut reg = stvec::read();
        reg.set_trap_mode(stvec::TrapMode::Direct);
        reg.set_address(trap_from_kernel as *const () as usize);
        unsafe { stvec::write(reg) };
    }

    #[inline]
    fn switch_context(
        current_context: *mut Self::TaskContext,
        next_context: *mut Self::TaskContext,
    ) {
        context::switch_context(current_context, next_context);
    }
}

/// 用于 M 模式下时钟中断到来时的辅助空间
///
/// [0..2]: 用来临时存放寄存器
/// [3]: 指向对应 CPU 的 CLINT MTIMECMP
/// [4]: 请求时钟中断的间隔
static mut TIMER_SCRATCH: [[u64; 5]; MAX_CPU_COUNT] = [[0; 5]; MAX_CPU_COUNT];

pub fn init_timer(hart_id: usize) {
    // 间隔多少时钟周期请求一次时钟中断
    let interval = CLOCK_CYCLE / 1000 * TIMER_INTERVAL;
    let mtimecmp = CLINT_ADDR + 0x4000 + 8 * hart_id;
    unsafe {
        TIMER_SCRATCH[hart_id][3] = mtimecmp as u64;
        TIMER_SCRATCH[hart_id][4] = interval as u64;
        mscratch::write(TIMER_SCRATCH[hart_id].as_ptr() as usize);
        // 设置时钟中断处理函数
        unsafe extern "C" {
            fn timer_trap();
        }
        mtvec::write(mtvec::Mtvec::new(
            timer_trap as *const () as usize,
            mtvec::TrapMode::Direct,
        ));
        // 开启 M 模式中断
        let mut reg_mstatus = mstatus::read();
        reg_mstatus.set_mie(true);
        mstatus::write(reg_mstatus);
        // 请求时钟周期
        // 第一次设置 mtimecmp 时，需要在 mtime 的基础上加上间隔
        let mtime = core::ptr::read_volatile((CLINT_ADDR + 0xBFF8) as *mut u64);
        core::ptr::write_volatile(mtimecmp as *mut u64, mtime + interval as u64);
        // M 模式接收时钟中断
        let mut reg_mie = mie::read();
        reg_mie.set_mtimer(true);
        mie::write(reg_mie);
        // 一定要在设置完 mtimecmp 之后再开启时钟中断，否则可能会开完中断立刻触发时钟中断、
        // 导致 mstatus 寄存器中的 MPP 被设置为 M，最后导致 mret 到 M 态时出现异常
    }
}
