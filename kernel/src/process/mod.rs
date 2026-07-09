pub mod context;
pub mod cpu;
pub mod elf;
pub mod kthread;
pub mod mm;
pub mod schedule;
pub mod sleep;
mod syscall;
pub mod task;

use crate::{
    arch::IrqArch,
    driver::{self, sifive_test::ShutdownReason},
    error::SystemError,
    exception::InterruptArch,
    fs::{
        ROOT_FS,
        file::File,
        vfs::{VirtualIndexNode, interface::FileType, lookup},
    },
    mm::mem_space::MemorySpace,
    println,
    process::{
        context::TrapContext, cpu::CPUManager, elf::load_elf, kthread::spawn_kthread,
        schedule::TaskScheduler, sleep::WaitQueue, task::TaskControlBlock,
    },
    sync::spin::{SpinMutex, SpinMutexGuard},
    utils::RecycleAllocator,
};
use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
    vec,
};
use core::{
    cell::SyncUnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};
use lazy_static::lazy_static;

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
            tls_size: SyncUnsafeCell::new(None),
            wait_queue: WaitQueue::new(),
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
    /// 注册一个进程
    pub fn register<F>(f: F) -> Arc<ProcessControlBlock>
    where
        F: FnOnce(usize) -> ProcessControlBlock,
    {
        let pid = PID_ALLOCATOR.fetch_add(1, Ordering::Relaxed);
        let process = Arc::new(f(pid));
        PROCESS_TABLE.lock().insert(pid, Arc::downgrade(&process));
        process
    }

    /// 启动初始进程
    pub fn init_process(path: &str) -> Result<(), SystemError> {
        let file = lookup(ROOT_FS.root(), path).ok_or(SystemError::ENOENT)?;
        if file.metadata().file_type != FileType::File {
            return Err(SystemError::EISDIR);
        }
        let mut elf_data = vec![0u8; file.metadata().size];
        file.read_at(0, &mut elf_data);
        let (memory_space, tls_size, entry_point) = load_elf(elf_data.as_slice())?;
        let cwd = lookup(ROOT_FS.root(), "root").ok_or(SystemError::ENOENT)?;
        if cwd.metadata().file_type != FileType::Directory {
            return Err(SystemError::ENOTDIR);
        }
        let process = ProcessManager::register(|pid| ProcessControlBlock {
            pid,
            tls_size: SyncUnsafeCell::new(tls_size),
            wait_queue: WaitQueue::new(),
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

        let task = TaskControlBlock::new(process.clone(), entry_point);
        // 初始进程不传递参数
        unsafe {
            let (_, ustack_high) = process.ustack_vaddr(task.id);
            // 初始时 ustack 已经被清零了，所以这里直接设置栈顶指针即可
            task.trap_context()
                .set_ustack(ustack_high - core::mem::size_of::<usize>());
        }
        TaskScheduler::push(task);
        Ok(())
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
    ///
    /// 这里用 [SyncUnsafeCell] 试图提供一种内部可变性，因为我们在 exec 时需要修改这个值
    tls_size: SyncUnsafeCell<Option<usize>>,
    /// 用于等待子进程
    pub wait_queue: WaitQueue,
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

    pub fn tls_size(&self) -> Option<usize> {
        unsafe { *self.tls_size.get() }
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
