#include "../string.h"
#include "../sys/process.h"
#include "../types.h"

extern int main(void);

extern char __tls_base;
extern char __tls_data_end;
extern char __tls_bss_end;

static void __tls_init(void) {
  usize tdata_size = (usize)(&__tls_data_end - &__tls_base);
  usize tbss_size = (usize)(&__tls_bss_end - &__tls_data_end);

  if (tdata_size == 0 && tbss_size == 0) {
    return;
  }

  char *tp;
  __asm__ volatile("mv %0, tp" : "=r"(tp));

  if (tdata_size != 0) {
    memcpy(tp, &__tls_base, (u32)tdata_size);
  }
  if (tbss_size != 0) {
    memset(tp + tdata_size, 0, (u32)tbss_size);
  }
}

__attribute__((section(".text.entry"))) __attribute__((used)) void
_start(void) {
  __tls_init();
  int ret = main();
  exit(ret);
}
