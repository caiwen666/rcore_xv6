#include "lib/dirent.h"
#include "lib/errno.h"
#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"

int main(int argc, char **argv) {
  const char *path = argc >= 2 ? argv[1] : ".";

  DIR *dir = opendir(path);
  if (dir == NULL) {
    printf("ls: cannot open '%s': %s\n", path, strerror(errno));
    return 1;
  }

  struct dirent *ent;
  while ((ent = readdir(dir)) != NULL) {
    /* 默认隐藏以 '.' 开头的条目，对齐 POSIX ls 的默认行为 */
    if (ent->d_name[0] == '.') {
      continue;
    }
    printf("%s", ent->d_name);
    if (ent->d_type == DT_DIR) {
      printf("/");
    }
    printf("  ");
  }
  printf("\n");

  closedir(dir);
  return 0;
}
