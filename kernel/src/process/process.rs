use crate::{
    fs::{file::File, vfs::VirtualIndexNode},
    process::task::TaskControlBlock,
    sync::spin::{SpinMutex, SpinMutexGuard},
    utils::RecycleAllocator,
};
use alloc::{sync::Arc, vec::Vec};

pub struct ProcessControlBlock {
    pub pid: usize,
    inner: SpinMutex<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    /// 任务列表
    ///
    /// 列表中的每个元素相当于是一个槽位，随着线程的创建，这个列表会越来越大。
    /// 当线程退出时，其对应的槽位不会被回收，而是会被标记为 None。
    /// 在创建线程时会优先查看任务列表中是否已有空闲槽位。
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    pub avail_task_id: RecycleAllocator,
    pub cwd: Option<VirtualIndexNode>,
    pub fd_table: Vec<Option<Arc<dyn File>>>,
    pub avail_fd: RecycleAllocator,
}

impl ProcessControlBlock {
    pub fn lock(&self) -> SpinMutexGuard<'_, ProcessControlBlockInner> {
        self.inner.lock()
    }

    /// 创建一个内核进程
    pub fn new_kernel() -> Self {
        Self {
            // 内核进程的 pid 固定为 0
            pid: 0,
            inner: SpinMutex::new(
                ProcessControlBlockInner {
                    tasks: Vec::new(),
                    avail_task_id: RecycleAllocator::new(),
                    cwd: None,
                    fd_table: Vec::new(),
                    avail_fd: RecycleAllocator::new(),
                },
                "kernel_process_inner",
            ),
        }
    }

    pub fn cwd(&self) -> VirtualIndexNode {
        self.lock().cwd.as_ref().cloned().unwrap()
    }

    pub fn set_cwd(&self, cwd: VirtualIndexNode) {
        let mut inner = self.lock();
        inner.cwd = Some(cwd);
    }
}
