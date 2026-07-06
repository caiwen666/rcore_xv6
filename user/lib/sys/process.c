#include "process.h"
#include "syscall.h"

void exit(int code) {
  syscall1(SYS_EXIT, (usize)(u8)code);
}
