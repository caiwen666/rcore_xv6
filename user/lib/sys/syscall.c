#include "syscall.h"

isize syscall0(usize id) {
  register isize a0 asm("a0") = 0;
  register usize a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
  return a0;
}

isize syscall1(usize id, usize arg0) {
  register isize a0 asm("a0") = arg0;
  register usize a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a7) : "memory");
  return a0;
}

isize syscall2(usize id, usize arg0, usize arg1) {
  register isize a0 asm("a0") = arg0;
  register usize a1 asm("a1") = arg1;
  register usize a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a7) : "memory");
  return a0;
}

isize syscall3(usize id, usize arg0, usize arg1, usize arg2) {
  register isize a0 asm("a0") = arg0;
  register usize a1 asm("a1") = arg1;
  register usize a2 asm("a2") = arg2;
  register usize a7 asm("a7") = id;

  asm volatile("ecall" : "+r"(a0) : "r"(a1), "r"(a2), "r"(a7) : "memory");
  return a0;
}

isize syscall4(usize id, usize arg0, usize arg1, usize arg2, usize arg3) {
  register isize a0 asm("a0") = arg0;
  register usize a1 asm("a1") = arg1;
  register usize a2 asm("a2") = arg2;
  register usize a3 asm("a3") = arg3;
  register usize a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a7)
               : "memory");
  return a0;
}

isize syscall5(usize id, usize arg0, usize arg1, usize arg2, usize arg3,
               usize arg4) {
  register isize a0 asm("a0") = arg0;
  register usize a1 asm("a1") = arg1;
  register usize a2 asm("a2") = arg2;
  register usize a3 asm("a3") = arg3;
  register usize a4 asm("a4") = arg4;
  register usize a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a7)
               : "memory");
  return a0;
}

isize syscall6(usize id, usize arg0, usize arg1, usize arg2, usize arg3,
               usize arg4, usize arg5) {
  register isize a0 asm("a0") = arg0;
  register usize a1 asm("a1") = arg1;
  register usize a2 asm("a2") = arg2;
  register usize a3 asm("a3") = arg3;
  register usize a4 asm("a4") = arg4;
  register usize a5 asm("a5") = arg5;
  register usize a7 asm("a7") = id;

  asm volatile("ecall"
               : "+r"(a0)
               : "r"(a1), "r"(a2), "r"(a3), "r"(a4), "r"(a5), "r"(a7)
               : "memory");
  return a0;
}
