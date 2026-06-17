pub mod context;
pub mod cpu;
pub mod kthread;
pub mod mm;
pub mod schedule;
pub mod task;
pub mod timer;

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    fs::{
        ROOT_FS,
        file::File,
        vfs::{VirtualIndexNode, lookup},
    },
    mm::{address::VirtAddr, mem_space::MemorySpace},
    process::{cpu::CPUManager, schedule::TaskScheduler, task::TaskControlBlock},
    sync::spin::{SpinMutex, SpinMutexGuard},
    utils::RecycleAllocator,
};
use alloc::{
    sync::{Arc, Weak},
    vec::Vec,
};
use lazy_static::lazy_static;
use xmas_elf::ElfFile;

lazy_static! {
    static ref KERNEL_PROCESS: Arc<ProcessControlBlock> = ProcessManager::init_kernel_process();
    static ref KERNEL_PROCESS_RESOURCE: Arc<SpinMutex<ProcessResource>> =
        ProcessManager::init_kernel_process_resource();
}
static PID_ALLOCATOR: AtomicUsize = AtomicUsize::new(1);

pub struct ProcessManager;

impl ProcessManager {
    /// 将会初始化内核进程
    pub fn init() {
        // 触发懒加载
        let _ = KERNEL_PROCESS.clone();
    }

    /// 初始化内核进程
    fn init_kernel_process() -> Arc<ProcessControlBlock> {
        Arc::new(ProcessControlBlock {
            // 内核进程的 pid 固定为 0
            pid: 0,
            inner: SpinMutex::new(
                ProcessControlBlockInner {
                    status: ProcessStatus::Running,
                },
                "kernel_process_inner",
            ),
        })
    }

    /// 初始化内核进程资源
    fn init_kernel_process_resource() -> Arc<SpinMutex<ProcessResource>> {
        Arc::new(SpinMutex::new(
            ProcessResource {
                memory_space: None,
                tasks: Vec::new(),
                avail_task_id: RecycleAllocator::new(),
                cwd: None,
                fd_table: Vec::new(),
                avail_fd: RecycleAllocator::new(),
            },
            "kernel_process_resource",
        ))
    }
}

impl ProcessManager {
    /// 获取当前进程
    #[expect(unused)]
    pub fn current() -> Arc<ProcessControlBlock> {
        let task = CPUManager::current_task().unwrap();
        task.process().clone()
    }

    /// 获取当前进程资源
    pub fn current_resource() -> Arc<SpinMutex<ProcessResource>> {
        let task = CPUManager::current_task().unwrap();
        task.process_resource().clone()
    }
}

impl ProcessManager {
    /// 根据 elf 文件创建一个进程，并为进程启动一个线程，并将进程加入到调度循环中。
    ///
    /// # Panics
    ///
    /// 如果 elf 文件解析失败，则 panic
    pub fn new_elf_process(elf_data: &[u8]) -> Arc<ProcessControlBlock> {
        let elf =
            ElfFile::new(elf_data).unwrap_or_else(|e| panic!("Failed to parse elf file: {}", e));
        let memory_space = MemorySpace::from_elf(&elf);
        let process = Arc::new(ProcessControlBlock {
            pid: PID_ALLOCATOR.fetch_add(1, Ordering::Relaxed),
            inner: SpinMutex::new(
                ProcessControlBlockInner {
                    status: ProcessStatus::Running,
                },
                "process_inner",
            ),
        });
        // 默认把工作目录设在 /root
        let cwd = lookup(ROOT_FS.root(), "root").unwrap();
        let process_resource = Arc::new(SpinMutex::new(
            ProcessResource {
                memory_space: Some(memory_space),
                tasks: Vec::new(),
                avail_task_id: RecycleAllocator::new(),
                cwd: Some(cwd),
                fd_table: Vec::new(),
                avail_fd: RecycleAllocator::new(),
            },
            "process_resource",
        ));
        process_resource.open_file("stdin");
        process_resource.open_file("stdout");
        // 标准错误流沿用标准输出
        process_resource.open_file("stdout");
        let task = TaskControlBlock::new(
            process.clone(),
            process_resource,
            VirtAddr::new(elf.header.pt2.entry_point() as usize),
        );

        TaskScheduler::push(task);
        process
    }
}

/// 进程基本信息
///
/// - 全局进程列表、父进程、进程下的所有线程的 TCB，这些会持有 PCB 的强引用
/// - 子进程 会持有 PCB 的弱引用
///
/// 当进程结束时，PCB 仍存在引用计数，不会被释放，直到父进程将其回收
pub struct ProcessControlBlock {
    pub pid: usize,
    inner: SpinMutex<ProcessControlBlockInner>,
}

pub enum ProcessStatus {
    /// 正在运行
    Running,
    /// 已经退出，携带退出码
    #[expect(unused)]
    Zombie(i32),
}

pub struct ProcessControlBlockInner {
    pub status: ProcessStatus,
}

impl ProcessControlBlock {
    pub fn inner(&self) -> SpinMutexGuard<'_, ProcessControlBlockInner> {
        self.inner.lock()
    }
}

/// 进程资源
///
/// 只有进程下的线程的 TCB 会持有进程资源的引用。当线程全部终止后，进程的资源自然
/// 就会被回收了。
pub struct ProcessResource {
    /// 如果是内核进程，则该字段为 None
    pub memory_space: Option<MemorySpace>,
    /// 任务列表
    ///
    /// 列表中的每个元素相当于是一个槽位，随着线程的创建，这个列表会越来越大。
    /// 当线程退出时，其对应的槽位不会被回收，而是会被标记为 None。
    /// 在创建线程时会优先查看任务列表中是否已有空闲槽位。
    pub tasks: Vec<Option<Weak<TaskControlBlock>>>,
    pub avail_task_id: RecycleAllocator,
    /// 只有内核进程在初始时为 None，其余情况下都不会为 None
    pub cwd: Option<VirtualIndexNode>,
    pub fd_table: Vec<Option<Arc<dyn File>>>,
    pub avail_fd: RecycleAllocator,
}

impl SpinMutex<ProcessResource> {
    pub fn cwd(&self) -> VirtualIndexNode {
        self.lock().cwd.as_ref().cloned().unwrap()
    }

    pub fn set_cwd(&self, cwd: VirtualIndexNode) {
        self.lock().cwd = Some(cwd);
    }
}
