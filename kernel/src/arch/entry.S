    # 内核整个的入口
    # linker.ld 会把下面的 _entry 放到 0x80000000 这个位置，
    # QEMU 就从这个位置开始执行
    # QEMU -bios none -kernel 时：a0 = hartid，a1 = DTB 物理地址
    .section .text.entry
    .globl _entry
_entry:
    # 先保存 DTB 物理地址，后面算栈会覆盖 a0/a1
    mv t0, a1
    # BOOT_STACK 符号被定义在 `cpu.rs` 中
    la sp, BOOT_STACK
    # 每个 CPU 核心启动时的初始栈的大小，与 `process/cpu.rs` 中 BOOT_STACK_SIZE 一致
    li a0, 1024 * 1024
    csrr a1, mhartid
    # 栈从高地址往低地址延申，sp 初始时放在高地址位置上
    addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0
    # 将 DTB 物理地址作为第一个参数传给 init_cpu
    mv a0, t0
    call init_cpu
