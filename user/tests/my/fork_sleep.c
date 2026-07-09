#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"
#include "lib/sys/process.h"

int tag = 0;

int main() {
  tag = 1;
  printf("hello from parent, tag: %d\n", tag);
  int pid = fork();
  if (pid == 0) {
    while (tag != 5) {
      printf("hello from child, tag: %d\n", tag);
      tag++;
      char *args[] = {"3", NULL};
      int pid = fork();
      if (pid == 0) {
        if (execv("/root/bin/sleep", args) < 0) {
          printf("execv failed: %s\n", strerror(errno));
          exit(1);
        }
      } else {
        waitpid(pid, NULL, 0);
      }
    }
  } else {
    printf("child pid: %d, waiting child to exit\n", pid);
    int status;
    waitpid(pid, &status, 0);
    printf("child exited with status: %d\n", status);
    while (tag != 5) {
      printf("hello from parent, tag: %d\n", tag);
      tag++;
      char *args[] = {"1", NULL};
      int pid = fork();
      if (pid == 0) {
        if (execv("/root/bin/sleep", args) < 0) {
          printf("execv failed: %s\n", strerror(errno));
          exit(1);
        }
      } else {
        waitpid(pid, NULL, 0);
      }
    }
  }
  return tag;
}