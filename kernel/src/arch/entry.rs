core::arch::global_asm!(include_str!("entry.S"));

use crate::{arch::fdt, driver::cpu::online_cpu_count, kernel_main};
use core::sync::atomic::{AtomicI8, AtomicUsize, Ordering};
use riscv::register::{
    Permission as PmpPermission, Range as PmpRange, mcounteren as RiscvMcounteren,
    medeleg as RiscvMedeleg, mepc as RiscvMepc, mhartid as RiscvMhartid, mideleg as RiscvMideleg,
    mstatus as RiscvMStatus, pmpaddr0 as RiscvPmpaddr0, pmpcfg0 as RiscvPmpcfg0, satp as RiscvSatp,
    sie as RiscvSie,
};

#[unsafe(no_mangle)]
extern "C" fn init_cpu(dtb_pa: usize) -> ! {
    let hart_id = RiscvMhartid::read();
    unsafe { super::register::tp::write_tp(hart_id) };

    // 设置成 -1 可以防止 EARLY_INIT_READY 本身进了 bss 段
    static EARLY_INIT_READY: AtomicI8 = AtomicI8::new(-1);
    if hart_id == 0 {
        // 只由 CPU0 来清空 BSS 段
        unsafe extern "C" {
            fn sbss();
            fn ebss();
        }
        ((sbss as *const () as usize)..(ebss as *const () as usize)).for_each(|a| unsafe {
            (a as *mut u8).write_volatile(0);
        });
        fdt::init_from_dtb(dtb_pa);
        EARLY_INIT_READY.store(1, Ordering::Release);
    } else {
        while EARLY_INIT_READY.load(Ordering::Acquire) != 1 {
            core::hint::spin_loop();
        }
    }

    let mut mstatus = RiscvMStatus::read();
    // 设置 mstatus 的 MPP 为 Supervisor，使得后续进入 Supervisor 模式
    mstatus.set_mpp(RiscvMStatus::MPP::Supervisor);
    // 默认不开启中断
    mstatus.set_sie(false);
    mstatus.set_spie(false);
    unsafe { RiscvMStatus::write(mstatus) };

    // 设置内核的入口
    unsafe { RiscvMepc::write(kernel_main as *const () as usize) };

    // 进入 Supervisor 模式后先关闭分页
    unsafe { RiscvSatp::set(RiscvSatp::Mode::Bare, 0, 0) };

    // QEMU 新版本对 PMP 审查更加严格
    // 我们需要把几乎所有的物理内存的所有权限全都开放
    unsafe {
        RiscvPmpaddr0::write(!0 >> 2);
        RiscvPmpcfg0::set_pmp(0, PmpRange::NAPOT, PmpPermission::RWX, false);
        core::arch::asm!("fence.i");
    }

    // 把所有的中断和异常都交给 Supervisor 模式处理
    let medeleg = RiscvMedeleg::Medeleg::from_bits(usize::MAX);
    unsafe { RiscvMedeleg::write(medeleg) };
    // M 模式的时钟中断（第 7 位）和机器软件中断（第 3 位，用于 IPI / TLB shootdown）
    // 都不委托给 S 模式，直接在 M 模式处理
    let mideleg = RiscvMideleg::Mideleg::from_bits(usize::MAX ^ (1 << 7) ^ (1 << 3));
    unsafe { RiscvMideleg::write(mideleg) };

    // 让 Supervisor 模式能够接收到外部中断/时钟中断和软件中断
    let mut sie = RiscvSie::read();
    sie.set_sext(true);
    sie.set_stimer(true);
    sie.set_ssoft(true);
    unsafe { RiscvSie::write(sie) };

    // 允许 S 模式访问到 time 寄存器
    let mut mcounteren = RiscvMcounteren::read();
    mcounteren.set_tm(true);
    unsafe { RiscvMcounteren::write(mcounteren) };

    // 初始化 M 模式的陷入处理（时钟中断 + IPI）
    super::interrupt::init_machine_trap(hart_id);

    // 等待所有 CPU 都初始化完成后再继续，确保所有 CPU 都设置好了中断之类的
    static READY_COUNT: AtomicUsize = AtomicUsize::new(0);
    READY_COUNT.fetch_add(1, Ordering::Release);
    while READY_COUNT.load(Ordering::Acquire) != online_cpu_count() {
        core::hint::spin_loop();
    }

    // 跳到内核的入口，并进入 Supervisor 模式
    unsafe { core::arch::asm!("mret", options(noreturn)) };
}
