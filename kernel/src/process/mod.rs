pub mod context;
pub mod cpu;
pub mod kthread;
pub mod mm;
#[expect(clippy::module_inception)]
pub mod process;
pub mod schedule;
pub mod task;

use crate::process::process::ProcessControlBlock;
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    static ref KERNEL_PROCESS: Arc<ProcessControlBlock> =
        Arc::new(ProcessControlBlock::new_kernel());
}

pub struct ProcessManager;

impl ProcessManager {
    /// 将会初始化内核进程
    pub fn init() {
        // 触发懒加载
        let _ = KERNEL_PROCESS.clone();
    }
}
