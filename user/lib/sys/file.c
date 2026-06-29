#include "file.h"
#include "syscall.h"

isize write(usize fd, const void *buf, usize len) {
  return syscall3(SYS_WRITE, fd, (usize)buf, len);
}

isize read(usize fd, void *buf, usize len) {
  return syscall3(SYS_READ, fd, (usize)buf, len);
}

isize chdir(const char *path) { return syscall1(SYS_CHDIR, (usize)path); }

isize getcwd(char *cwd) { return syscall1(SYS_GETCWD, (usize)cwd); }