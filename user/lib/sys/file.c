#include "file.h"
#include "syscall.h"

isize write(usize fd, const void *buf, usize len) {
  return syscall3(SYS_WRITE, fd, (usize)buf, len);
}
