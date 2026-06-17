extern int main(void);

__attribute__((section(".text.entry"))) __attribute__((used)) void
_start(void) {
  main();

  // 故意触发一个特权级指令，让内核 panic
  __asm__ volatile("wfi");
}
