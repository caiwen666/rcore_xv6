#include "lib/string.h"
#include "lib/sys/file.h"

int main(void) {
  char buf[] = "Hello, World! from user!\n";
  int len = strlen(buf);
  write(STDOUT, buf, len);
  return 0;
}
