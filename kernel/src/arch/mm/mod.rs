mod init;
mod pte;

use core::sync::atomic::{AtomicBool, Ordering};
use riscv::register::satp::{self, Satp};

use crate::{
    arch::{cpu::cpu_id, interrupt::TLB_SHOOTDOWN_ACK, mm::pte::Sv39PTE},
    driver::{
        CLINT_ADDR,
        cpu::{MAX_CPU_COUNT, online_cpu_mask},
    },
    mm::{MemoryManagementArch, mem_space::MemorySpace},
};

pub struct RiscV64MMArch;

impl MemoryManagementArch for RiscV64MMArch {
    type PTE = Sv39PTE;

    /// 页面大小为 4096
    const PAGE_SIZE_SHIFT: usize = 12;
    /// 三级页表
    const PAGE_LEVELS: usize = 3;
    /// 每个页表的页表项数量为 512
    const PTE_COUNT_SHIFT: usize = 9;
    /// 跳板占了 1 个页面
    const TRAMPOLINE_PAGE_COUNT: usize = 1;
    /// 虚拟内存地址有 39 位，但是 SV39 要求第 39 位 (1-base) 为 1 的时候剩余高位必须都为 1
    /// 为了简单起见我们只用 38 位
    const VADDR_BITS_COUNT: usize = 38;

    fn init() {
        init::init();
    }

    fn activate(space: &MemorySpace) {
        unsafe {
            riscv::register::satp::write(make_satp(space));
            core::arch::asm!("sfence.vma");
        }
    }

    #[inline]
    fn local_flush_tlb() {
        unsafe {
            core::arch::asm!("sfence.vma");
        };
    }

    unsafe fn tlb_shootdown() {
        let cpu_mask = online_cpu_mask();
        // SAFETY: 此时中断已经关闭
        let me = unsafe { cpu_id() };

        // 目前为了简单起见，只允许同一时间只有一个 CPU 发起 TLB shootdown 并等待结束
        static SHOOTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
        // 稳妥一点，这里用 acquire，释放锁的时候用 release
        while SHOOTDOWN_IN_PROGRESS
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }

        // 确保页表的修改在 IPI 发出之前对其他 CPU 可见
        // 前面对页表的修改是主存写，后面对 CLINT 写来发起中断是设备写，要用 o
        unsafe { core::arch::asm!("fence w,ow") };

        // 向每个目标 CPU 的 CLINT MSIP 寄存器写 1，触发机器软件中断（IPI）
        for hart in 0..MAX_CPU_COUNT {
            if (cpu_mask & (1 << hart)) == 0 || hart == me {
                continue;
            }
            let msip = CLINT_ADDR + 4 * hart;
            unsafe { core::ptr::write_volatile(msip as *mut u32, 1) };
        }

        let mut remaining = cpu_mask & !(1 << me);
        while remaining != 0 {
            for (hart, ack) in TLB_SHOOTDOWN_ACK.iter().enumerate() {
                if hart == me || (cpu_mask & (1 << hart)) == 0 {
                    continue;
                }
                if ack.load(Ordering::Relaxed) != 0 {
                    ack.store(0, Ordering::Relaxed);
                    remaining &= !(1 << hart);
                }
            }
            core::hint::spin_loop();
        }

        SHOOTDOWN_IN_PROGRESS.store(false, Ordering::Release);
    }
}

pub fn make_satp(space: &MemorySpace) -> Satp {
    let mut reg = satp::Satp::from_bits(0);
    reg.set_mode(satp::Mode::Sv39);
    // 当前实现直接把页表全刷了，所以 asid 无所谓
    reg.set_asid(0);
    reg.set_ppn(unsafe { space.table().paddr().inner() >> 12 });
    reg
}
