/**
 * rcore-xv6 进程相关系统调用
 */

#ifndef RCORE_XV6_PROCESS_H
#define RCORE_XV6_PROCESS_H

#include "../types.h"

/**
 * 终止当前进程
 * @param code 退出码，仅低 8 位有效
 */
void exit(int code);

/**
 * 创建子进程
 * @return 父进程中返回子进程 pid，子进程中返回 0；失败返回 -1 并设置 errno
 */
int fork(void);

/**
 * 睡眠指定时间
 * @param us 睡眠时间（微秒）
 * @return 若被提前唤醒则返回剩余未睡眠的微秒数，完全睡完返回 0
 */
isize sleep(usize us);

#endif
