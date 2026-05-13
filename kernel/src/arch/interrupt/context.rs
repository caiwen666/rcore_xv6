use crate::{mm::address::VirtAddr, process::schedule::task_entry};

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
        pub fn __switch(current_context: *mut TaskContext, next_context: *mut TaskContext);
    }
    unsafe { __switch(current_context, next_context) };
}
