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

/**
 * 打开文件
 * @param path 文件路径
 * @return 成功返回文件描述符（>= 0）；失败返回 -1 并设置 errno
 */
int open(const char *path);

/**
 * 关闭文件描述符
 * @param fd 文件描述符
 * @return 成功返回 0；失败返回 -1 并设置 errno
 */
int close(usize fd);

/**
 * 读取目录项（对齐 Linux getdents64 的简化版）
 *
 * 从目录描述符 fd 的当前位置开始，把若干条变长 dirent 连续写入 buf，
 * 直到下一条放不下或到达目录末尾。每条记录布局为：
 * d_ino(8) + d_off(8) + d_reclen(2) + d_type(1) + d_name + NUL，
 * d_reclen 已上对齐到 8，d_off 即下一条记录的偏移。
 *
 * @param fd 目录文件描述符
 * @param buf 存放 dirent 的缓冲区（建议 8 字节对齐）
 * @param count 缓冲区大小
 * @return 成功返回写入的字节数；0 表示已到达目录末尾；失败返回 -1 并设置 errno
 */
isize getdents(usize fd, void *buf, usize count);

#endif
