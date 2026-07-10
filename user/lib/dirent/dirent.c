#include "../dirent.h"
#include "../errno.h"
#include "../stdlib.h"
#include "../string.h"
#include "../sys/file.h"

/**
 * 与内核 sys_readdir 写入的变长记录布局一致（repr(C)）。
 * d_name 是柔性数组，起始偏移 = 8 + 8 + 2 + 1 = 19。
 *
 * 由于 DIR.buf 是 usize 数组（8 字节对齐），且每条记录的 d_reclen 都是 8 的倍数，
 * 所以把 (buf + pos) 强转为本结构体指针不会有未对齐访问。
 */
struct linux_dirent {
  usize d_ino;
  usize d_off;
  u16 d_reclen;
  u8 d_type;
  char d_name[];
};

DIR *opendir(const char *path) {
  int fd = open(path);
  if (fd < 0) {
    return NULL;
  }
  DIR *dir = malloc(sizeof(DIR));
  if (dir == NULL) {
    close((usize)fd);
    errno = ENOMEM;
    return NULL;
  }
  memset(dir, 0, sizeof(DIR));
  dir->fd = fd;
  return dir;
}

struct dirent *readdir(DIR *dir) {
  if (dir == NULL) {
    return NULL;
  }
  /* 当前缓冲区消费完，重新调用 getdents 填充 */
  if (dir->pos >= dir->valid) {
    isize n = getdents((usize)dir->fd, dir->buf, sizeof(dir->buf));
    if (n <= 0) {
      /* n == 0：目录已读完；n < 0：出错，errno 已由 getdents 设置 */
      return NULL;
    }
    dir->valid = (usize)n;
    dir->pos = 0;
  }
  struct linux_dirent *src =
      (struct linux_dirent *)((char *)dir->buf + dir->pos);
  dir->cur.d_ino = src->d_ino;
  dir->cur.d_off = src->d_off;
  dir->cur.d_reclen = src->d_reclen;
  dir->cur.d_type = src->d_type;
  safestrcpy(dir->cur.d_name, src->d_name, NAME_MAX + 1);
  dir->pos += src->d_reclen;
  return &dir->cur;
}

int closedir(DIR *dir) {
  if (dir == NULL) {
    errno = EINVAL;
    return -1;
  }
  int ret = close((usize)dir->fd);
  free(dir);
  return ret;
}
