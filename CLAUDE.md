# CLAUDE.md

## Code Review 注意点

执行 `/code-review` 或任何代码审查时,必须额外检查:

1. 如果一个函数会调用 `process/sleep` 内的线程睡眠基础设施，那么这个函数是会阻塞的。内核代码中调用会阻塞的函数时不能持有自旋锁。

2. 为了防止避免死锁，我们对锁的顺序有如下的要求：
- 对 ProcessControlBlock(PCB) 的 inner 进行加锁时，如果涉及到既对父进程的 PCB 加锁又对子进程的 PCB 加锁，必须是先对父进程加锁再对子进程加锁。
- 如果既对 ProcessControlBlock(PCB) 的 inner 加锁，又对 TaskControlBlock(TCB) 的 inner 加锁，必须是先对 PCB 加锁再对 TCB 加锁。