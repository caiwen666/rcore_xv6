#include "process.h"
#include "../errno.h"
#include "../stdlib.h"
#include "syscall.h"
#include "syscall_errno.h"

/** 与内核 exec 可接受的参数规模大致匹配 */
#define EXECV_MAX_ARGS 32

void exit(int code) { syscall1(SYS_EXIT, (usize)(u8)code); }

int fork(void) { return (int)syscall_errno(syscall0(SYS_FORK)); }

isize sleep(usize us) { return syscall_errno(syscall1(SYS_SLEEP, us)); }

int waitpid(int pid, int *status, int non_blocking) {
  return (int)syscall_errno(syscall3(SYS_WAITPID, (usize)pid, (usize)status,
                                     (usize)non_blocking));
}

int exec(const char *path, char *const argv[]) {
  return (int)syscall_errno(
      syscall2(SYS_EXEC, (usize)path, (usize)argv));
}

int execv(const char *path, char *const argv[]) {
  char *full_argv[EXECV_MAX_ARGS + 2];
  usize argc;

  if (path == 0 || path[0] == '\0') {
    errno = EINVAL;
    return -1;
  }

  full_argv[0] = (char *)path;
  argc = 1;

  if (argv != 0) {
    for (usize i = 0; argv[i] != 0; i++) {
      if (argc >= EXECV_MAX_ARGS + 1) {
        errno = EINVAL;
        return -1;
      }
      full_argv[argc++] = argv[i];
    }
  }
  full_argv[argc] = 0;

  return exec(path, full_argv);
}
