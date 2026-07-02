#ifndef RCORE_XV6_MM_H
#define RCORE_XV6_MM_H

#include "../types.h"

/**
 * 调整进程的堆大小
 * @param increment 堆大小的增量，如果大于 0 则扩容，小于 0 则缩容
 * @return 调整前堆的结束地址；失败返回 NULL 并设置 errno
 */
void *sbrk(isize increment);

#endif