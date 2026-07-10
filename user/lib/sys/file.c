#include "file.h"
#include "../stdlib.h"
#include "syscall.h"
#include "syscall_errno.h"

isize write(usize fd, const void *buf, usize len) {
  return syscall_errno(syscall3(SYS_WRITE, fd, (usize)buf, len));
}

isize read(usize fd, void *buf, usize len) {
  return syscall_errno(syscall3(SYS_READ, fd, (usize)buf, len));
}

int chdir(const char *path) {
  return (int)syscall_errno(syscall1(SYS_CHDIR, (usize)path));
}

char *getcwd(char *buf, usize size) {
  isize ret = syscall2(SYS_GETCWD, (usize)buf, size);
  if (ret < 0) {
    errno = (int)-ret;
    return NULL;
  }
  errno = 0;
  return (char *)ret;
}

int open(const char *path) {
  return (int)syscall_errno(syscall1(SYS_OPEN, (usize)path));
}

int close(usize fd) {
  return (int)syscall_errno(syscall1(SYS_CLOSE, fd));
}

isize getdents(usize fd, void *buf, usize count) {
  return syscall_errno(syscall3(SYS_READDIR, fd, (usize)buf, count));
}
