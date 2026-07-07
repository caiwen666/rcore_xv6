#include "lib/stdio.h"
#include "lib/sys/process.h"

int tag = 0;

int main() {
  tag = 1;
  printf("hello from parent, tag: %d\n", tag);
  int pid = fork();
  if (pid == 0) {
    while (tag != 5) {
      int interval = 1000 * 1000; // 1 seconds
      printf("hello from child, tag: %d\n", tag);
      tag++;
      sleep(interval);
    }
  } else {
    printf("child pid: %d, waiting child to exit\n", pid);
    int status;
    waitpid(pid, &status, 0);
    printf("child exited with status: %d\n", status);
    while (tag != 5) {
      printf("hello from parent, tag: %d\n", tag);
      tag++;
      int interval = 1000 * 1000; // 1 second
      sleep(interval);
    }
  }
  return tag;
}