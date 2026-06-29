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
 * @return 写入的字节数，如果写入失败，返回 -1
 */
isize write(usize fd, const void *buf, usize len);

/**
 * 读取文件
 * @param fd 文件描述符
 * @param buf 读取内容的缓冲区的指针
 * @param len 读取内容的缓冲区的长度
 * @return 读取的字节数，如果读取失败，返回 -1
 */
isize read(usize fd, void *buf, usize len);

/**
 * 切换当前工作目录
 * @param path 新的工作目录的路径
 * @return 0 表示成功，-1 表示失败
 */
isize chdir(const char *path);

/**
 * 获取当前工作目录
 * 调用时请确保缓冲区有足够的空间存放工作目录路径，否则会导致缓冲区溢出
 * @param cwd 存放工作目录路径的缓冲区的指针
 * @return 成功则返回工作目录路径的长度，失败则返回 -1
 */
isize getcwd(char *cwd);

#endif
