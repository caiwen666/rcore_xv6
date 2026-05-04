    # 内核整个的入口
    # linker.ld 会把下面的 _entry 放到 0x80000000 这个位置，
    # QEMU 就从这个位置开始执行
    .section .text.entry
    .globl _entry
_entry:
    # BOOT_STACK 符号被定义在 `cpu.rs` 中
    la sp, BOOT_STACK
    # 每个 CPU 核心启动时的初始栈的大小，与 `process/cpu.rs` 中保持一致
    li a0, 4096
    csrr a1, mhartid
    # 栈从高地址往低地址延申，sp 初始时放在高地址位置上
    addi a1, a1, 1
    mul a0, a0, a1
    add sp, sp, a0
    call init_cpu