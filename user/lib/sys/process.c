#include "process.h"
#include "../stdlib.h"
#include "syscall.h"
#include "syscall_errno.h"

void exit(int code) { syscall1(SYS_EXIT, (usize)(u8)code); }

int fork(void) { return (int)syscall_errno(syscall0(SYS_FORK)); }

isize sleep(usize us) { return syscall_errno(syscall1(SYS_SLEEP, us)); }

int waitpid(int pid, int *status, int non_blocking) {
  return (int)syscall_errno(syscall3(SYS_WAITPID, (usize)pid, (usize)status,
                                     (usize)non_blocking));
}
