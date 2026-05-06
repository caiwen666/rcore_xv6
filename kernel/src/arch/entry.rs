core::arch::global_asm!(include_str!("entry.S"));

use crate::kernel_main;
use riscv::register::{
    Permission as PmpPermission, Range as PmpRange, medeleg as RiscvMedeleg, mepc as RiscvMepc,
    mhartid as RiscvMhartid, mideleg as RiscvMideleg, mstatus as RiscvMStatus,
    pmpaddr0 as RiscvPmpaddr0, pmpcfg0 as RiscvPmpcfg0, satp as RiscvSatp, sie as RiscvSie,
};

#[unsafe(no_mangle)]
extern "C" fn init_cpu() -> ! {
    // 清空 BSS 段
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    ((sbss as *const () as usize)..(ebss as *const () as usize)).for_each(|a| unsafe {
        (a as *mut u8).write_volatile(0);
    });

    // 设置 mstatus 的 MPP 为 Supervisor，使得后续进入 Supervisor 模式
    let mut mstatus = RiscvMStatus::read();
    mstatus.set_mpp(RiscvMStatus::MPP::Supervisor);
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
    let mideleg = RiscvMideleg::Mideleg::from_bits(usize::MAX);
    unsafe { RiscvMideleg::write(mideleg) };

    // 让 Supervisor 模式能够接收到外部中断/时钟中断和软件中断
    let mut sie = RiscvSie::read();
    sie.set_sext(true);
    sie.set_stimer(true);
    sie.set_ssoft(true);
    unsafe { RiscvSie::write(sie) };

    // 初始化时钟中断
    // TODO

    // 将 cpu 的 id 写到 tp 寄存器上
    let hart_id = RiscvMhartid::read();
    unsafe { super::register::tp::write_tp(hart_id) };

    // 跳到内核的入口，并进入 Supervisor 模式
    unsafe { core::arch::asm!("mret", options(noreturn)) };
}
