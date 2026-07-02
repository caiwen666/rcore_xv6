#include "lib/stdio.h"

__thread int tls_var = 123;

int main(void) {
  char buf[] = "Hello, World! from user!";
  printf("%s\n", buf);
  printf("tls_var: %d\n", tls_var);
  tls_var = 456;
  printf("tls_var: %d\n", tls_var);
  return 0;
}
