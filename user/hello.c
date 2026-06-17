#include "lib/syscall.h"

int main(void) {
  int last = 1;
  for (int i = 0; i < 5; i++) {
    last = syscall0(last);
  }
  return last / (last - 6);
}
