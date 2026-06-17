    # 从内核态陷入内核态，不用放到 .trampoline 里
    .globl trap_from_kernel
    .balign 4
trap_from_kernel:
    # 直接大力保存所有寄存器到栈上
    addi sp, sp, -32*8
    .set n, 0
    .rept 32
        SAVE_REG %n
        .set n, n+1
    .endr

    call kernel_trap_handler
    
    ld x0, 0(sp)
    ld x1, 8(sp)
    ld x2, 16(sp)
    ld x3, 24(sp)
    # x4 是 tp。经过上下文的切换，现在可能已经在别的 cpu 上面调度了，所以不需要恢复
    # ld x4, 32(sp)
    .set n, 5
    .rept 27
        LOAD_REG %n
        .set n, n+1
    .endr

    addi sp, sp, 32*8

    sret

