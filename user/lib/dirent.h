/**
 * rcore-xv6 目录读取
 *
 * 提供类似 POSIX 的 opendir / readdir / closedir 接口。
 * 底层基于 SYS_READDIR（getdents64 风格），用户态在此之上做缓冲。
 */

#ifndef RCORE_XV6_DIRENT_H
#define RCORE_XV6_DIRENT_H

#include "types.h"

/**
 * d_type 取值，对齐 Linux <dirent.h> 的 DT_* 宏。
 * 当前内核只会产生 DT_DIR 与 DT_REG。
 */
#define DT_UNKNOWN 0
#define DT_FIFO 1
#define DT_CHR 2
#define DT_DIR 4
#define DT_BLK 6
#define DT_REG 8
#define DT_LNK 10
#define DT_SOCK 12
#define DT_WHT 14

/** 文件名最大长度（不含结尾 NUL） */
#define NAME_MAX 255

/**
 * 单条目录项。readdir 返回指向 DIR 内部该结构体的指针。
 *
 * 注意：内核 SYS_READDIR 写入的是变长记录，readdir 会将其整理成此定长结构，
 * 两者布局不必逐字节一致。
 */
struct dirent {
  /** inode 编号 */
  usize d_ino;
  /** 下一条记录在目录中的偏移（= 内核 offset_cookie） */
  usize d_off;
  /** 本条记录的字节长度 */
  u16 d_reclen;
  /** 文件类型，取 DT_* 之一 */
  u8 d_type;
  /** 文件名，以 '\0' 结尾 */
  char d_name[NAME_MAX + 1];
};

/** 目录流，由 opendir 返回 */
typedef struct {
  /** 目录的文件描述符 */
  int fd;
  /** getdents 缓冲区，按 8 字节对齐，可直接按记录强转解析 */
  usize buf[64];
  /** buf 中有效数据的字节数 */
  usize valid;
  /** 当前消费到 buf 中的位置 */
  usize pos;
  /** 最近一次 readdir 的结果，readdir 返回指向它的指针 */
  struct dirent cur;
} DIR;

/**
 * 打开目录流
 * @param path 目录路径
 * @return 成功返回目录流指针；失败返回 NULL 并设置 errno
 */
DIR *opendir(const char *path);

/**
 * 读取下一条目录项
 * @param dir 由 opendir 返回的目录流
 * @return 成功返回指向 DIR 内部 dirent 的指针；到达末尾或出错时返回 NULL
 *         （到达末尾不设置 errno，出错时设置 errno）
 */
struct dirent *readdir(DIR *dir);

/**
 * 关闭目录流
 * @param dir 目录流；传 NULL 会返回 EINVAL
 * @return 成功返回 0；失败返回 -1 并设置 errno
 */
int closedir(DIR *dir);

#endif
