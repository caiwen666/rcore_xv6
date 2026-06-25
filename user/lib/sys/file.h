/**
 * rcore-xv6 文件读写相关
 */

#ifndef RCORE_XV6_FILE_H
#define RCORE_XV6_FILE_H

#include "../types.h"

#define STDIN 0
#define STDOUT 1
#define STDERR 2

/**
 * 写入文件
 * @param fd 文件描述符
 * @param buf 写入内容的缓冲区的指针
 * @param len 写入内容的缓冲区的长度
 * @return 写入的字节数，如果写入失败，返回 -1
 */
isize write(usize fd, const void *buf, usize len);

#endif
