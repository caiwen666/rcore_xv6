/**
 * rcore-xv6 错误码（与内核 SystemError 的 POSIX 编号一致）
 */

#ifndef RCORE_XV6_ERRNO_H
#define RCORE_XV6_ERRNO_H

#define EPERM 1
#define ENOENT 2
#define EINTR 4
#define EBADF 9
#define ECHILD 10
#define ENOMEM 12
#define EFAULT 14
#define ENOTDIR 20
#define EINVAL 22
#define ERANGE 34
#define ENAMETOOLONG 36

/** 当前线程最近一次失败系统调用的错误码 */
extern __thread int errno;

#endif
