/**
 * rcore-xv6 文件读写相关
 */

#ifndef RCORE_XV6_FILE_H
#define RCORE_XV6_FILE_H

#include "../types.h"

/**
 * 写入文件
 * @param fd 文件描述符
 * @param buf 写入内容的缓冲区的指针
 * @param len 写入内容的缓冲区的长度
 * @return 写入的字节数；失败返回 -1 并设置 errno
 */
isize write(usize fd, const void *buf, usize len);

/**
 * 读取文件
 * @param fd 文件描述符
 * @param buf 读取内容的缓冲区的指针
 * @param len 读取内容的缓冲区的长度
 * @return 读取的字节数；失败返回 -1 并设置 errno
 */
isize read(usize fd, void *buf, usize len);

/**
 * 切换当前工作目录
 * @param path 新的工作目录的路径
 * @return 0 表示成功；-1 表示失败并设置 errno
 */
int chdir(const char *path);

/**
 * 获取当前工作目录
 * @param buf 存放路径的缓冲区
 * @param size 缓冲区大小（含结尾 '\0'）
 * @return 成功返回 buf；失败返回 NULL 并设置 errno
 */
char *getcwd(char *buf, usize size);

#endif
