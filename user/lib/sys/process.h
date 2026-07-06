/**
 * rcore-xv6 进程相关系统调用
 */

#ifndef RCORE_XV6_PROCESS_H
#define RCORE_XV6_PROCESS_H

/**
 * 终止当前进程
 * @param code 退出码，仅低 8 位有效
 */
void exit(int code);

#endif
