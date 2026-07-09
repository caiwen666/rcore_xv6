use crate::{
    error::SystemError,
    fs::vfs::{interface::FileType, lookup},
    mm::{
        address::VirtAddr,
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission},
    },
    process::{
        ProcessManager,
        context::{ArchTrapContext, TrapContext},
        elf::load_elf,
        mm::USER_HEAP_START,
        task::TaskControlBlock,
    },
};
use alloc::vec;
use alloc::vec::Vec;
use syscall_macros::syscall;

#[syscall(name = "SYS_EXEC", id = 9)]
fn sys_exec(args: [usize; 6]) -> Result<usize, SystemError> {
    const MAX_STR_LEN: usize = 100;
    let path_vaddr = VirtAddr::new(args[0]);
    let mut args_vaddr = VirtAddr::new(args[1]);
    let process = ProcessManager::current_process();
    let process_inner = process.inner();
    // 只支持单线程的进程执行 exec
    if process_inner.tasks.len() != 1 {
        return Err(SystemError::EPERM);
    }
    let memory_space = process_inner.memory_space.as_ref().unwrap();
    let path = memory_space.copyin_str(path_vaddr, MAX_STR_LEN)?;
    if path.is_empty() {
        return Err(SystemError::EINVAL);
    }
    let mut args = Vec::new();
    if !args_vaddr.is_null() {
        loop {
            let arg_vaddr = VirtAddr::new(memory_space.copyin(args_vaddr)?);
            if arg_vaddr.is_null() {
                break;
            }
            let arg = memory_space.copyin_str(arg_vaddr, MAX_STR_LEN)?;
            args.push(arg);
            args_vaddr += core::mem::size_of::<usize>();
        }
    }

    let cwd = process_inner.cwd.clone().unwrap();
    drop(process_inner);
    let file = lookup(cwd, &path).ok_or(SystemError::ENOENT)?;
    if file.metadata().file_type == FileType::Directory {
        return Err(SystemError::EISDIR);
    }
    let mut elf_data = vec![0u8; file.metadata().size];
    file.read_at(0, &mut elf_data);

    let mut process_inner = process.inner();
    let (memory_space, tls_size, entry_point) = load_elf(elf_data.as_slice())?;
    // 替换掉当前的内存空间
    process_inner.memory_space = Some(memory_space);
    // 重新设置堆
    process_inner.heap_size = 0;
    let memory_space = process_inner.memory_space.as_mut().unwrap();
    memory_space.push(MemoryArea::new(
        VirtAddr::new(USER_HEAP_START),
        0,
        MemoryPermission::Readable | MemoryPermission::Writable | MemoryPermission::UserAccessible,
        MemoryAreaType::Private,
        "heap",
    ));
    // 重新设置 tls 区域大小
    unsafe {
        *process.tls_size.get() = tls_size;
    }
    // 初始化线程的内存
    let task = ProcessManager::current_task();
    TaskControlBlock::init_memory(&process, memory_space, task.id);
    // 拿到 trap 上下文的物理地址
    let (trap_context_paddr, _) = memory_space
        .translate_vaddr(process.trap_context_vaddr(task.id))
        .unwrap();
    let (_, kstack_high) = task.kstack.range();
    let (_, ustack_high) = process.ustack_vaddr(task.id);
    let tls_base = process.tls_vaddr(task.id).unwrap_or(VirtAddr::new(0));

    // 向用户栈写入启动参数
    // 先计算整个数据的大小：参数个数 + 每个参数的指针 + 空指针 + 每个参数的字符串（带 \0）
    let data_size = core::mem::size_of::<usize>() * (args.len() + 2)
        + args.iter().map(|arg| arg.len() + 1).sum::<usize>();
    // riscv 架构需要 16 字节对齐
    let padding = data_size.div_ceil(16) * 16 - data_size;
    // 计算用户栈的指针
    let mut ustack_ptr = ustack_high - padding;
    let mut args_ustack_ptr = Vec::with_capacity(args.len());
    for arg in args {
        ustack_ptr -= arg.len() + 1;
        args_ustack_ptr.push(ustack_ptr);
        memory_space.copyout_str(ustack_ptr, arg)?;
    }
    ustack_ptr -= core::mem::size_of::<usize>();
    memory_space.copyout(ustack_ptr, 0usize)?;
    for ptr in args_ustack_ptr.iter().rev() {
        ustack_ptr -= core::mem::size_of::<usize>();
        memory_space.copyout(ustack_ptr, ptr.inner())?;
    }
    ustack_ptr -= core::mem::size_of::<usize>();
    memory_space.copyout(ustack_ptr, args_ustack_ptr.len())?;

    unsafe {
        // 修改 trap 上下文的物理地址
        *task.trap_context_paddr.get() = Some(trap_context_paddr);
        // 重新设置 trap 上下文
        task.with_trap_context(move |trap_context| {
            let mut new_trap_context = ArchTrapContext::new(kstack_high);
            new_trap_context
                .set_ustack(ustack_ptr)
                .set_pc(entry_point)
                .set_tls_base(tls_base);
            *trap_context = new_trap_context;
        });
    }
    Ok(0)
}
