use crate::{
    driver::MEMORY_AREAS,
    mm::{
        KERNEL_SPACE, PhysMemoryAreaKind,
        address::{PhysAddr, VirtAddr},
        allocator::FRAME_ALLOCATOR,
        mem_space::{MemoryArea, MemoryAreaType, MemoryPermission},
    },
};

unsafe extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss();
    fn ebss();
    fn ekernel();
}

pub fn init() {
    let stext = stext as *const () as usize;
    let etext = etext as *const () as usize;
    let srodata = srodata as *const () as usize;
    let erodata = erodata as *const () as usize;
    let sdata = sdata as *const () as usize;
    let edata = edata as *const () as usize;
    let sbss = sbss as *const () as usize;
    let ebss = ebss as *const () as usize;
    let ekernel = ekernel as *const () as usize;

    // 允许 Supervisor 模式访问用户空间
    unsafe { riscv::register::sstatus::set_sum() };
    // 由于当前内核启动并不太复杂，所以可以直接上 buddy 内存分配
    // 寻找主存，并配置全局页帧分配器
    let main_memory_area = MEMORY_AREAS
        .iter()
        .find(|area| area.kind == PhysMemoryAreaKind::MainMemory)
        .expect("Main memory area not configured");
    let main_memory_end = main_memory_area.base + main_memory_area.size;
    // 页帧分配器只管理从 ekernel 开始的物理内存
    FRAME_ALLOCATOR.lock().add_area(
        PhysAddr::new(ekernel as *const () as usize),
        main_memory_end,
    );
    // 从此刻开始，内核也能用堆了

    // KERNEL_SPACE 是 lazy_static，只有在第一次访问时才会初始化
    // 此时是第一次访问，而此时也初始化好内存分配器了，所以 KERNEL_SPACE 能正确初始化
    let mut kernel_space = KERNEL_SPACE.lock();

    for area in MEMORY_AREAS {
        match area.kind {
            PhysMemoryAreaKind::Device => {
                let mem_area = MemoryArea::new(
                    VirtAddr::new(area.base.inner()),
                    area.size,
                    MemoryPermission::Readable | MemoryPermission::Writable,
                    MemoryAreaType::Identical,
                    area.name,
                );
                kernel_space.push(mem_area);
            }
            // 后面再映射，这里先跳过
            PhysMemoryAreaKind::MainMemory => continue,
        }
    }

    let text_area = MemoryArea::new(
        VirtAddr::new(stext),
        // linker.ld 中都已经 4k 对齐了，所以无需考虑上取整
        etext - stext,
        MemoryPermission::Readable | MemoryPermission::Executable,
        MemoryAreaType::Identical,
        "kernel_text",
    );
    kernel_space.push(text_area);

    let rodata_area = MemoryArea::new(
        VirtAddr::new(srodata),
        erodata - srodata,
        // BOOT_STACK 链接在 .rodata 段内；启动栈必须可写。整段 rodata 映射为 RW
        //（后续可改为仅映射栈页或把栈挪到 .bss）。
        MemoryPermission::Readable | MemoryPermission::Writable,
        MemoryAreaType::Identical,
        "kernel_rodata",
    );
    kernel_space.push(rodata_area);

    let data_area = MemoryArea::new(
        VirtAddr::new(sdata),
        edata - sdata,
        MemoryPermission::Readable | MemoryPermission::Writable,
        MemoryAreaType::Identical,
        "kernel_data",
    );
    kernel_space.push(data_area);

    let bss_area = MemoryArea::new(
        VirtAddr::new(sbss),
        ebss - sbss,
        MemoryPermission::Readable | MemoryPermission::Writable,
        MemoryAreaType::Identical,
        "kernel_bss",
    );
    kernel_space.push(bss_area);

    // 然后再映射剩余物理内存区域
    let phys_mem_area = MemoryArea::new(
        VirtAddr::new(ekernel),
        main_memory_end.inner() - ekernel,
        MemoryPermission::Readable | MemoryPermission::Writable,
        MemoryAreaType::Identical,
        "kernel_phys_mem",
    );
    kernel_space.push(phys_mem_area);
    kernel_space.map_trampoline();
}
