mod init;
mod pte;

use core::sync::atomic::Ordering;
use riscv::register::satp::{self, Satp};

use crate::{
    arch::{
        cpu::cpu_id,
        interrupt::{TLB_SHOOTDOWN_ACK, TLB_SHOOTDOWN_REQUEST},
        mm::pte::Sv39PTE,
    },
    driver::{CLINT_ADDR, cpu::ONLINE_CPU_COUNT},
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
        let online_cpu_count = ONLINE_CPU_COUNT.load(Ordering::Relaxed);
        // SAFETY: 此时中断已经关闭
        let me = unsafe { cpu_id() };

        // 使用 release，确保其他线程直到读到这个 req 值，就能读到本次的页表修改
        let req = TLB_SHOOTDOWN_REQUEST
            .fetch_add(1, Ordering::Release)
            .wrapping_add(1);

        // 确保对 req 的修改在 IPI 发出之前对其他 CPU 可见
        // 前面对 req 的修改是主存写，后面对 CLINT 写来发起中断是设备写，要用 o
        unsafe { core::arch::asm!("fence w,ow") };

        // 向每个目标 CPU 的 CLINT MSIP 寄存器写 1，触发机器软件中断（IPI）
        for hart in 0..online_cpu_count {
            if hart == me {
                continue;
            }
            let msip = CLINT_ADDR + 4 * hart;
            unsafe { core::ptr::write_volatile(msip as *mut u32, 1) };
        }

        let mut remaining = ((1 << online_cpu_count) - 1) & !(1 << me);
        while remaining != 0 {
            #[expect(clippy::needless_range_loop)]
            for hart in 0..online_cpu_count {
                if hart == me {
                    continue;
                }
                let ack = TLB_SHOOTDOWN_ACK[hart].load(Ordering::Relaxed);
                // ack >= req 说明目标 CPU 已经 ack 了本次 TLB shootdown 请求
                if ack.wrapping_sub(req) as isize >= 0 {
                    remaining &= !(1 << hart);
                }
            }
            core::hint::spin_loop();
        }
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
