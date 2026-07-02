#ifndef RCORE_XV6_SYSCALL_ERRNO_H
#define RCORE_XV6_SYSCALL_ERRNO_H

#include "../errno.h"
#include "../types.h"

/**
 * 将内核返回的原始 syscall 结果转换为 POSIX 语义。
 * 失败时设置 errno 并返回 -1；成功时清除 errno 并返回原值。
 */
static inline isize syscall_errno(isize ret) {
  if (ret < 0) {
    errno = (int)-ret;
    return -1;
  }
  errno = 0;
  return ret;
}

#endif
