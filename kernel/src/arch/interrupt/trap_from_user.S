    .section .trampoline, "ax"
    .balign 4
    .globl trap_from_user
trap_from_user:
    # 将用户栈指针寄存在 sscratch 中，同时把 sscratch 之前存放的指向当前任务的
    # trap 上下文的指针取出来放到 sp 中
    csrrw sp, sscratch, sp
    # 开始写入 trap 上下文
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 4
    .rept 28
        SAVE_REG %n
        .set n, n+1
    .endr
    csrr t0, sepc
    sd t0, 32*8(sp)
    csrr t1, sscratch
    sd t1, 2*8(sp)

    # 将 hartid 保存在 tp 中
    ld tp, 35*8(sp)

    # 获得内核页表
    ld t0, 33*8(sp)

    # 设置内核栈指针
    # 每次进入内核栈，栈指针都设置为内核栈的栈顶
    ld sp, 34*8(sp)

    # 切换到内核页表
    csrw satp, t0
    sfence.vma

    # 这里不能直接把 user_trap_handler 的地址 la 过来，因为汇编器在这里会用相对定位
    # 但是 .trampoline 段在内核跑起来之后就不再是原来代码上的相对位置关系
    # 所以需要先 .quad 把 user_trap_handler 的绝对地址写到 .trampoline 的某个地方
    # 然后再间接跳转
    la t0, user_trap_handler_ptr
    ld t1, 0(t0)
    jr t1

    .balign 8
user_trap_handler_ptr:
    .quad user_trap_handler

    .balign 4
    .globl __return_to_user
__return_to_user:
    # a0（参数一）：trap 上下文在用户空间的地址
    # a1（参数二）：用户空间的 satp 值
    
    # 切换到用户空间
    csrw satp, a1
    sfence.vma

    # 将 trap 上下文的地址放在 sscratch 中
    csrw sscratch, a0

    # 将当前 cpu 的 id 存入 trap 上下文
    sd tp, 35*8(a0)

    # 设置 sepc
    ld t0, 32*8(a0)
    csrw sepc, t0
    
    # 设置 SPP 为 0，回到用户态
    li t1, 1 << 8
    csrc sstatus, t1
    # 设置 SPIE 为 1，回到用户态后开中断
    li t1, 1 << 5
    csrs sstatus, t1

    # 恢复通用寄存器
    mv sp, a0
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 4
    .rept 28
        LOAD_REG %n
        .set n, n+1
    .endr
    ld sp, 2*8(sp)
    sret