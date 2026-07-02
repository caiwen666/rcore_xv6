#include "mm.h"
#include "../stdlib.h"
#include "syscall.h"
#include "syscall_errno.h"

void *sbrk(isize increment) {
  isize ret = syscall1(SYS_SBRK, (usize)increment);
  if (ret < 0) {
    errno = (int)-ret;
    return NULL;
  }
  errno = 0;
  return (void *)ret;
}
