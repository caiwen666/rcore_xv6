#include "lib/stdio.h"
#include "lib/sys/process.h"

int tag = 0;

int main() {
  tag = 1;
  printf("hello from parent\ntag: %d\n", tag);
  int pid = fork();
  if (pid == 0) {
    while (tag != 5) {
      int interval = 1000 * 1000 * 3; // 3 seconds
      printf("hello from child\ntag: %d\n", tag);
      tag++;
      sleep(interval);
    }
  } else {
    printf("child pid: %d\n", pid);
    while (tag != 5) {
      printf("hello from parent\ntag: %d\n", tag);
      tag++;
      int interval = 1000 * 1000; // 1 second
      sleep(interval);
    }
  }
  return 0;
}