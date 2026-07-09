#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"
#include "lib/sys/process.h"

int main() {
  printf("hello, world\n");
  int pid = fork();
  if (pid == 0) {
    printf("hello from child\n");
    if (exec("/root/bin/tests/my/fork_sleep", NULL) < 0) {
      printf("exec failed: %s\n", strerror(errno));
      exit(1);
    }
  } else {
    printf("hello from parent\n");
  }
  printf("parent process exited\n");
  return 0;
}