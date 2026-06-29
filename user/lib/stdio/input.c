#include "../stdio.h"
#include "../stdlib.h"
#include "../sys/file.h"

int getchar(void) {
  unsigned char c;
  if (read(STDIN, &c, 1) == 0)
    return EOF;
  return c;
}

int getline(int fd, char *buf, int max) {
  int i, c, cc;

  for (i = 0; i + 1 < max;) {
    cc = read(fd, &c, 1);
    if (cc == 0)
      return EOF;
    buf[i++] = c;
    if (c == '\n' || c == '\r')
      break;
  }
  buf[i] = '\0';
  return i;
}