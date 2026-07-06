pub mod context;
pub mod cpu;
pub mod kthread;
pub mod mm;
pub mod schedule;
pub mod sleep;
mod syscall;
pub mod task;

use crate::{
    arch::{IrqArch, MMArch},
    driver::{self, sifive_test::ShutdownReason},
    exception::InterruptArch,
    fs::{
        ROOT_FS,
        file::File,
        vfs::{VirtualIndexNode, lookup},
    },
    mm::{MemoryManagementArch, address::VirtAddr, mem_space::MemorySpace},
    println,
    process::{
        cpu::CPUManager, kthread::spawn_kthread, schedule::TaskScheduler, task::TaskControlBlock,
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
    utils::RecycleAllocator,
};
use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
};
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use xmas_elf::{ElfFile, program::Type};

lazy_static! {
    static ref KERNEL_PROCESS: Arc<ProcessControlBlock> = ProcessManager::init_kernel_process();
    static ref PROCESS_TABLE: SpinMutex<BTreeMap<usize, Weak<ProcessControlBlock>>> =
        SpinMutex::new(BTreeMap::new(), "process_table");
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
            tls_size: None,
            inner: SpinMutex::new(
                ProcessControlBlockInner {
                    status: ProcessStatus::Running,
                    memory_space: None,
                    tasks: RecycleAllocator::new(),
                    cwd: None,
                    fd_table: RecycleAllocator::new(),
                    heap_size: 0,
                    parent: None,
                    children: BTreeMap::new(),
                    exited_children: BTreeMap::new(),
                },
                "kernel_process_inner",
            ),
        })
    }
}

impl ProcessManager {
    /// 获取当前任务
    ///
    /// # Panics
    ///
    /// - 如果当前没有任务，则 panic
    pub fn current_task() -> Arc<TaskControlBlock> {
        let interrupted = IrqArch::get_interrupt_state();
        IrqArch::disable_interrupt();
        let cpu = unsafe { CPUManager::current_cpu() };
        cpu.spinning_state.push_lock(interrupted);
        let task = cpu.current_task.clone().unwrap();
        if cpu.spinning_state.pop_lock() {
            IrqArch::enable_interrupt();
        }
        task
    }

    /// 获取当前进程
    pub fn current_process() -> Arc<ProcessControlBlock> {
        let task = Self::current_task();
        task.process().clone()
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
        // 解析 tls
        let tls_size = elf
            .program_iter()
            .find(|ph| ph.get_type().unwrap() == Type::Tls)
            .map(|ph| (ph.mem_size() as usize).div_ceil(MMArch::PAGE_SIZE) * MMArch::PAGE_SIZE);
        // 默认把工作目录设在 /root
        let cwd = lookup(ROOT_FS.root(), "root").unwrap();
        let process = Arc::new(ProcessControlBlock {
            pid: PID_ALLOCATOR.fetch_add(1, Ordering::Relaxed),
            tls_size,
            inner: SpinMutex::new(
                ProcessControlBlockInner {
                    status: ProcessStatus::Running,
                    memory_space: Some(memory_space),
                    tasks: RecycleAllocator::new(),
                    cwd: Some(cwd),
                    fd_table: RecycleAllocator::new(),
                    heap_size: 0,
                    parent: None,
                    children: BTreeMap::new(),
                    exited_children: BTreeMap::new(),
                },
                "process_inner",
            ),
        });
        process.open_file("stdin");
        process.open_file("stdout");
        // 标准错误流沿用标准输出
        process.open_file("stdout");
        let task = TaskControlBlock::new(
            process.clone(),
            VirtAddr::new(elf.header.pt2.entry_point() as usize),
        );

        TaskScheduler::push(task);
        PROCESS_TABLE
            .lock()
            .insert(process.pid, Arc::downgrade(&process));
        process
    }
}

/// ProcessControlBlock(PCB)
///
/// PCB 放在堆上面，其强引用只会出现在如下几个地方：
///
/// - 进程包含的线程
/// - 父进程
/// - 子进程
pub struct ProcessControlBlock {
    pub pid: usize,
    /// 进程需要的 tls 区域的大小，按页大小对齐
    pub tls_size: Option<usize>,
    inner: SpinMutex<ProcessControlBlockInner>,
}

pub enum ProcessStatus {
    /// 正在运行
    Running,
    /// 已经有线程将进程退出，u8 为退出代码
    Exiting(u8),
}

pub struct ProcessControlBlockInner {
    pub status: ProcessStatus,
    /// 父进程，孤儿进程为 None
    pub parent: Option<Arc<ProcessControlBlock>>,
    /// 子进程列表
    pub children: BTreeMap<usize, Arc<ProcessControlBlock>>,
    /// 子进程退出时，会将其 pid 和退出状态保存到这里，等待父进程回收
    ///
    /// map 的 key 为子进程的 pid，value 为子进程的退出代码
    pub exited_children: BTreeMap<usize, u8>,
    /// 如果是内核进程，则该字段为 None
    pub memory_space: Option<MemorySpace>,
    pub tasks: RecycleAllocator<Weak<TaskControlBlock>>,
    /// 只有内核进程在初始时为 None，其余情况下都不会为 None
    pub cwd: Option<VirtualIndexNode>,
    pub fd_table: RecycleAllocator<Arc<dyn File>>,
    // heap 在内存空间对应的区域的大小是经过对齐的，这里记录一下对齐前的真实大小，确保
    // sbrk 返回的地址是对的
    pub heap_size: isize,
}

impl ProcessControlBlock {
    pub fn inner(&self) -> SpinMutexGuard<'_, ProcessControlBlockInner> {
        self.inner.lock()
    }

    /// **该函数会对 inner 加锁**
    pub fn cwd(&self) -> VirtualIndexNode {
        self.inner().cwd.as_ref().unwrap().clone()
    }

    /// **该函数会对 inner 加锁**
    pub fn set_cwd(&self, cwd: VirtualIndexNode) {
        let mut inner = self.inner();
        inner.cwd = Some(cwd);
    }
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        let mut inner = PROCESS_TABLE.lock();
        inner.remove(&self.pid);
        // 所有进程都退出的话，启动一个内核线程，负责退出整个系统
        if inner.is_empty() {
            spawn_kthread(|| {
                println!(
                    "[kernel] All processes have exited, waiting for all kernel threads to exit..."
                );
                loop {
                    // 循环检查是否还有内核线程在运行
                    let inner = KERNEL_PROCESS.inner();
                    if inner.tasks.len() == 1 {
                        // 就剩当前这个内核线程了，关机
                        println!("[kernel] All kernel threads have exited, shutting down...");
                        driver::SIFIVE_TEST.shutdown(ShutdownReason::Normal, 0);
                    }
                    drop(inner);
                    ProcessManager::yield_current();
                }
            });
        }
    }
}
