#include "mm.h"
#include "syscall.h"

void *sbrk(isize increment) { return (void *)syscall1(SYS_SBRK, increment); }