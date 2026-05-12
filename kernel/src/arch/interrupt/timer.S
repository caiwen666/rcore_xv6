    .globl timer_trap
    # mtvec BASE 在 Direct 模式下须 4 字节对齐，否则 riscv::mtvec::write 会失败 */
    .balign 4
timer_trap:
    # mscratch 指向 mod.rs 中对应 CPU 的 TIMER_SCRATCH 数组
    csrrw a0, mscratch, a0
    # 把 a1 到 a3 临时存放起来
    sd a1, 0(a0)
    sd a2, 8(a0)
    sd a3, 16(a0)

    ld a1, 24(a0)   # CLINT_MTIMECMP(hart)
    ld a2, 32(a0)   # interval
    ld a3, 0(a1)    # mtimecmp 当前值
    add a3, a3, a2
    sd a3, 0(a1)

    # 然后把 M 模式的时钟中断，以软件中断的方式转发到 S 模式
    li a1, 2
    csrrs x0, sip, a1

    # 恢复
    ld a3, 16(a0)
    ld a2, 8(a0)
    ld a1, 0(a0)
    csrrw a0, mscratch, a0

    mret