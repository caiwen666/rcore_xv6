/**
 * rcore-xv6 系统调用
 * a7 存放系统调用 id，a0 到 a6 作为参数，调用后 a0 作为返回值
 */

#ifndef RCORE_XV6_SYSCALL_H
#define RCORE_XV6_SYSCALL_H

static inline long syscall0(long id) {
  register long a0 asm("a0") = 0;
  register long a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
  return a0;
}

static inline long syscall1(long id, long arg0) {
  register long a0 asm("a0") = arg0;
  register long a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
  return a0;
}

static inline long syscall2(long id, long arg0, long arg1) {
  register long a0 asm("a0") = arg0;
  register long a1 asm("a1") = arg1;
  register long a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a7) : "memory");
  return a0;
}

static inline long syscall3(long id, long arg0, long arg1, long arg2) {
  register long a0 asm("a0") = arg0;
  register long a1 asm("a1") = arg1;
  register long a2 asm("a2") = arg2;
  register long a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a2), "r"(a7) : "memory");
  return a0;
}

static inline long syscall4(long id, long arg0, long arg1, long arg2,
                            long arg3) {
  register long a0 asm("a0") = arg0;
  register long a1 asm("a1") = arg1;
  register long a2 asm("a2") = arg2;
  register long a3 asm("a3") = arg3;
  register long a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a7)
               : "memory");
  return a0;
}

static inline long syscall5(long id, long arg0, long arg1, long arg2, long arg3,
                            long arg4) {
  register long a0 asm("a0") = arg0;
  register long a1 asm("a1") = arg1;
  register long a2 asm("a2") = arg2;
  register long a3 asm("a3") = arg3;
  register long a4 asm("a4") = arg4;
  register long a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a7)
               : "memory");
  return a0;
}

static inline long syscall6(long id, long arg0, long arg1, long arg2, long arg3,
                            long arg4, long arg5) {
  register long a0 asm("a0") = arg0;
  register long a1 asm("a1") = arg1;
  register long a2 asm("a2") = arg2;
  register long a3 asm("a3") = arg3;
  register long a4 asm("a4") = arg4;
  register long a5 asm("a5") = arg5;
  register long a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a5), "r"(a7)
               : "memory");
  return a0;
}

#endif
