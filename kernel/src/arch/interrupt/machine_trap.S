    .globl m_trap
    # mtvec BASE 在 Direct 模式下须 4 字节对齐，否则 mtvec::write 会失败 */
    .balign 4
m_trap:
    # mscratch 指向 mod.rs 中对应 CPU 的 M_SCRATCH 数组
    csrrw a0, mscratch, a0
    # 把 a1 到 a3 临时存放起来
    sd a1, 0(a0)
    sd a2, 8(a0)
    sd a3, 16(a0)

    # 读取陷入原因，最高位为 1 表示是中断
    csrr a1, mcause
    # 最高位为 0，则 a1 按补码解释成正数，直接跳过
    bgez a1, m_trap_restore

    # 屏蔽掉最高位的中断标志，得到 cause code
    andi a1, a1, 0xff
    li a2, 7
    # 时钟中断
    beq a1, a2, m_trap_timer
    # 软件中断
    li a2, 3
    beq a1, a2, m_trap_soft

    j m_trap_restore

m_trap_timer:
    ld a1, 24(a0)   # CLINT_MTIMECMP(hart)
    ld a2, 32(a0)   # interval
    ld a3, 0(a1)    # mtimecmp 当前值
    add a3, a3, a2
    sd a3, 0(a1)

    # 然后把 M 模式的时钟中断，以软件中断的方式转发到 S 模式
    li a1, 2
    csrrs x0, sip, a1
    j m_trap_restore

m_trap_soft:
    # 收到其他 CPU 发来的 TLB shootdown IPI，立刻刷新当前 CPU 的 TLB
    sfence.vma
    # 清除当前 CPU 的 MSIP
    ld a1, 40(a0)   # CLINT_MSIP(hart)
    sw x0, 0(a1)
    # 原子地递增本 CPU 的 ACK 计数，通知发起方已经完成刷新
    ld a1, 48(a0)   # &TLB_SHOOTDOWN_ACK[hart]
    li a2, 1
    amoadd.d x0, a2, (a1)
    j m_trap_restore

m_trap_restore:
    # 恢复
    ld a3, 16(a0)
    ld a2, 8(a0)
    ld a1, 0(a0)
    csrrw a0, mscratch, a0

    mret
