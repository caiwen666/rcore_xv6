#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/sys/process.h"

int main(int argc, char **argv) {
  char *end;
  long seconds;

  if (argc != 2) {
    printf("usage: %s seconds\n", argv[0]);
    return 1;
  }

  seconds = strtol(argv[1], &end, 10);
  if (end == argv[1] || *end != '\0' || seconds < 0) {
    printf("sleep: invalid seconds\n");
    return 1;
  }

  sleep((usize)seconds * 1000000);
  return 0;
}
