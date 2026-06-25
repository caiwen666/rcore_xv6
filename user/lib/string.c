#include "string.h"

usize strlen(const char *s) {
  usize len = 0;
  while (s[len] != '\0') {
    len++;
  }
  return len;
}
