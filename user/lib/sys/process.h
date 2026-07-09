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

/**
 * 等待子进程退出
 * @param pid 要等待的子进程 pid；为 0 时等待任意子进程
 * @param status 存放子进程退出码的指针，可为 NULL
 * @param non_blocking 0 表示阻塞等待；非 0 表示非阻塞，无已退出子进程时返回 0
 * @return 成功返回已退出子进程的 pid；非阻塞且无已退出子进程时返回 0；
 *         失败返回 -1 并设置 errno
 */
int waitpid(int pid, int *status, int non_blocking);

/**
 * 执行新程序，替换当前进程映像
 * @param path 可执行文件路径
 * @param argv 参数列表，以 NULL 结尾；可为 NULL
 * @return 成功时不返回；失败返回 -1 并设置 errno
 */
int exec(const char *path, char *const argv[]);

/**
 * 执行新程序，替换当前进程映像
 * @param path 可执行文件路径
 * @param argv 除程序名外的参数列表，以 NULL 结尾；可为 NULL
 * @return 成功时不返回；失败返回 -1 并设置 errno
 */
int execv(const char *path, char *const argv[]);

#endif
