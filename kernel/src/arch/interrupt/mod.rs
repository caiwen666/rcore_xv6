mod context;
mod handler;

use crate::{
    arch::{MMArch, mm::make_satp},
    driver::{
        CLINT_ADDR,
        cpu::{CLOCK_CYCLE, MAX_CPU_COUNT},
    },
    exception::{InterruptArch, timer::TIMER_INTERVAL},
    mm::{MemoryManagementArch, address::VirtAddr},
    process::{ProcessManager, cpu::CPUManager},
};
use core::sync::atomic::AtomicUsize;
use riscv::register::{mie, mscratch, mstatus, mtvec, satp::Satp, sstatus, stvec};

core::arch::global_asm!(include_str!("machine_trap.S"));

pub struct RiscV64InterruptArch;

impl InterruptArch for RiscV64InterruptArch {
    type TaskContext = context::TaskContext;
    type TrapContext = context::TrapContext;

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
    unsafe fn switch_context(
        current_context: *mut Self::TaskContext,
        next_context: *mut Self::TaskContext,
    ) {
        context::switch_context(current_context, next_context);
    }

    fn return_to_user() -> ! {
        RiscV64InterruptArch::disable_interrupt();
        let cpu = unsafe { CPUManager::current_cpu() };
        assert!(
            cpu.spinning_state.count == 0,
            "Cannot return to user when spinning state is not 0."
        );

        // 准备函数
        unsafe extern "C" {
            fn strampoline();
            fn __return_to_user();
            fn trap_from_user();
        }
        let trampoline_code_base =
            (1 << MMArch::VADDR_BITS_COUNT) - MMArch::TRAMPOLINE_PAGE_COUNT * MMArch::PAGE_SIZE;
        // __return_to_user 函数在跳板部分的地址
        let f_ptr = trampoline_code_base
            + (__return_to_user as *const () as usize - strampoline as *const () as usize);
        let f: fn(VirtAddr, Satp) -> ! = unsafe { core::mem::transmute(f_ptr) };

        // 准备 satp
        let satp;
        // 涉及的资源有点多，直接用一个代码块包裹，不用再一个个 drop 了
        {
            let process = ProcessManager::current_process();
            let inner = process.inner();
            let memory_space = inner.memory_space.as_ref().unwrap();
            satp = make_satp(memory_space);
        }

        let mut reg_stvec = stvec::read();
        reg_stvec.set_trap_mode(stvec::TrapMode::Direct);
        reg_stvec.set_address(
            trampoline_code_base
                + (trap_from_user as *const () as usize - strampoline as *const () as usize),
        );
        unsafe { stvec::write(reg_stvec) };

        let trap_vaddr;
        {
            let current_task = cpu.current_task.as_ref().unwrap();
            let process = current_task.process();
            trap_vaddr = process.trap_context_vaddr(current_task.id);
        }
        f(trap_vaddr, satp)
    }
}

/// 全局 TLB shootdown 请求编号的计数器
pub static TLB_SHOOTDOWN_REQUEST: AtomicUsize = AtomicUsize::new(0);

/// 每个 CPU 已经 ack 的 TLB shootdown 的请求编号
pub static TLB_SHOOTDOWN_ACK: [AtomicUsize; MAX_CPU_COUNT] =
    [const { AtomicUsize::new(0) }; MAX_CPU_COUNT];

/// 用于 M 模式陷入时的辅助空间（每个 CPU 一个）
///
/// [0..2]: 用来临时存放寄存器 a1/a2/a3
/// [3]: 指向对应 CPU 的 CLINT MTIMECMP
/// [4]: 请求时钟中断的间隔
/// [5]: 指向对应 CPU 的 CLINT MSIP，用于 TLB shootdown 的 IPI
/// [6]: 指向对应 CPU 的 TLB_SHOOTDOWN_ACK 计数器
/// [7]: 指向当前全局 TLB shootdown 请求编号的计数器
static mut M_SCRATCH: [[u64; 8]; MAX_CPU_COUNT] = [[0; 8]; MAX_CPU_COUNT];

/// 初始化 M 模式的陷入处理，时钟中断和 IPI
///
/// 每个 CPU 核心都需要调用一次
pub fn init_machine_trap(hart_id: usize) {
    // 间隔多少时钟周期请求一次时钟中断
    let interval = CLOCK_CYCLE / 1000 * TIMER_INTERVAL;
    let mtimecmp = CLINT_ADDR + 0x4000 + 8 * hart_id;
    let msip = CLINT_ADDR + 4 * hart_id;
    unsafe {
        M_SCRATCH[hart_id][3] = mtimecmp as u64;
        M_SCRATCH[hart_id][4] = interval as u64;
        M_SCRATCH[hart_id][5] = msip as u64;
        M_SCRATCH[hart_id][6] = TLB_SHOOTDOWN_ACK[hart_id].as_ptr() as u64;
        M_SCRATCH[hart_id][7] = TLB_SHOOTDOWN_REQUEST.as_ptr() as u64;
        mscratch::write(M_SCRATCH[hart_id].as_ptr() as usize);
        // 设置 M 模式的陷入处理函数
        unsafe extern "C" {
            fn m_trap();
        }
        mtvec::write(mtvec::Mtvec::new(
            m_trap as *const () as usize,
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
        // M 模式接收时钟中断和软件中断，其中软件中断目前就设置为 IPI
        let mut reg_mie = mie::read();
        reg_mie.set_mtimer(true);
        reg_mie.set_msoft(true);
        mie::write(reg_mie);
        // 一定要在设置完 mtimecmp 之后再开启时钟中断，否则可能会开完中断立刻触发时钟中断、
        // 导致 mstatus 寄存器中的 MPP 被设置为 M，最后导致 mret 到 M 态时出现异常
    }
}
