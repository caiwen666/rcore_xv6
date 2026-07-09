use crate::{
    arch::mm::make_satp,
    mm::{KERNEL_SPACE, address::VirtAddr},
    process::schedule::task_entry,
};
use riscv::register::satp::Satp;

core::arch::global_asm!(include_str!("switch.S"));

#[derive(Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    // 只考虑被调用者保存寄存器
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl crate::process::context::TaskContext for TaskContext {
    fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    fn new(kernel_stack: VirtAddr) -> Self {
        Self {
            ra: task_entry as *const () as usize,
            sp: kernel_stack.inner(),
            s: [0; 12],
        }
    }
}

pub fn switch_context(current_context: *mut TaskContext, next_context: *mut TaskContext) {
    unsafe extern "C" {
        fn __switch(current_context: *mut TaskContext, next_context: *mut TaskContext);
    }
    unsafe { __switch(current_context, next_context) };
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapContext {
    // x 和 sepc 是可被任意修改的
    pub x: [usize; 32],
    pub sepc: usize,
    // 下面三个是固定的，TrapContext 初始化之后就不动了
    pub kernel_satp: Satp,
    pub kernel_sp: usize,
    // 这个是在内核态和用户态之间切换时自动维护的
    pub hartid: usize,
}

impl crate::process::context::TrapContext for TrapContext {
    fn new(kstack: VirtAddr) -> Self {
        Self {
            x: [0; 32],
            sepc: 0,
            kernel_satp: make_satp(&KERNEL_SPACE.lock()),
            kernel_sp: kstack.inner(),
            hartid: 0,
        }
    }

    fn pc(&self) -> VirtAddr {
        VirtAddr::new(self.sepc)
    }

    fn set_pc(&mut self, pc: VirtAddr) -> &mut Self {
        self.sepc = pc.inner();
        self
    }

    fn set_ustack(&mut self, ustack: VirtAddr) -> &mut Self {
        self.x[2] = ustack.inner();
        self
    }

    fn set_tls_base(&mut self, tls_base: VirtAddr) -> &mut Self {
        self.x[4] = tls_base.inner();
        self
    }

    fn set_return_value(&mut self, return_value: usize) -> &mut Self {
        self.x[10] = return_value;
        self
    }

    fn set_kernel_sp(&mut self, kernel_sp: VirtAddr) -> &mut Self {
        self.kernel_sp = kernel_sp.inner();
        self
    }
}
