use syscall_macros::syscall;

use crate::{
    arch::MMArch,
    error::SystemError,
    mm::{MemoryManagementArch, address::VirtAddr},
    process::{cpu::CPUManager, mm::USER_HEAP_START},
};

#[syscall(name = "SYS_SBRK", id = 3)]
fn sys_sbrk(args: [usize; 6]) -> Result<usize, SystemError> {
    let increment = args[0] as isize;

    let task = CPUManager::current_task().unwrap();
    let resource = task.process_resource();
    let mut resource_guard = resource.lock();

    let result = USER_HEAP_START + resource_guard.heap_size as usize;
    if increment == 0 {
        return Ok(result);
    }
    if resource_guard.heap_size.checked_add(increment).is_none()
        || resource_guard.heap_size + increment < 0
    {
        return Err(SystemError::ENOMEM);
    }

    resource_guard.heap_size += increment;
    let heap_size = resource_guard.heap_size;
    let memory_space = resource_guard.memory_space.as_mut().unwrap();
    // 寻找到 heap 内存区域
    let heap_area = memory_space
        .find_area(VirtAddr::new(USER_HEAP_START))
        .unwrap();
    let heap_mem_size = heap_area.size();

    // heap_size 和 heap_mem_size 之间会有一个因为页对齐而产生的差距
    // 如果由于页对齐而多余的大小正好满足扩充需求，则直接返回
    // 如果缩小完了之后还需要原来的最后一页，则直接返回
    // 其余情况，则需要调整内存区域大小
    // 下面更直观的是 resource_guard.heap_size <= heap_size - MMArch::PAGE_SIZE，做了移项，防止溢出
    if heap_size as usize > heap_mem_size || heap_size as usize + MMArch::PAGE_SIZE <= heap_mem_size
    {
        memory_space.resize(VirtAddr::new(USER_HEAP_START), heap_size as usize);
    }

    Ok(result)
}
