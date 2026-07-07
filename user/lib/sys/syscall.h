/**
 * rcore-xv6 系统调用
 * a7 存放系统调用 id，a0 到 a6 作为参数，调用后 a0 作为返回值
 */

#ifndef RCORE_XV6_SYSCALL_H
#define RCORE_XV6_SYSCALL_H

#include "../types.h"

#define SYS_WRITE 0
#define SYS_READ 1
#define SYS_CHDIR 2
#define SYS_SBRK 3
#define SYS_GETCWD 4
#define SYS_EXIT 5
#define SYS_FORK 6
#define SYS_SLEEP 7
#define SYS_WAITPID 8

isize syscall0(usize id);
isize syscall1(usize id, usize arg0);
isize syscall2(usize id, usize arg0, usize arg1);
isize syscall3(usize id, usize arg0, usize arg1, usize arg2);
isize syscall4(usize id, usize arg0, usize arg1, usize arg2, usize arg3);
isize syscall5(usize id, usize arg0, usize arg1, usize arg2, usize arg3,
               usize arg4);
isize syscall6(usize id, usize arg0, usize arg1, usize arg2, usize arg3,
               usize arg4, usize arg5);

#endif
