#include "../lib/stdio.h"
#include "../lib/string.h"
#include "../lib/sys/file.h"
#include "cmd.h"
#include "common.h"
#include "debug.h"
#include "lineedit.h"
#include "parse.h"

char cwd[MAXLINE];
static char buf[MAXLINE];

int main(void) {
  // sh 为第一个进程，操作系统启动该程序的时候会自动打开 stdin、stdout 和 stderr
  if (getcwd(cwd) < 0) {
    // 可能内核给的初始 cwd 被删了（虽然不太可能），遇到这种情况就改为根目录
    strcpy(cwd, "/");
    chdir("/");
    printf("sh: getcwd failed, changed to root directory\n");
  }

  // TODO 设置关闭控制台 ICANON

  while (true) {
    if (getcmd(buf, sizeof(buf)) < 0)
      break;

    if (buf[0] == 0)
      continue;

    if (buf[0] == 'c' && buf[1] == 'd' && buf[2] == ' ') {
      if (chdir(buf + 3) < 0)
        printf("sh: cannot cd %s\n", buf + 3);
      else if (getcwd(cwd) < 0) {
        // 成功切换目录了，但是获取路径又失败，这种情况比较罕见，遇到了就回退到根目录
        chdir("/");
        strcpy(cwd, "/");
        printf("sh: getcwd failed, changed to root directory\n");
      }
      continue;
    }

    struct cmd *cmd = parsecmd(buf);
    if (cmd == NULL)
      continue;
    printcmd(cmd);
    // TODO
    // if (fork1() == 0)
    //   runcmd(cmd);
    // wait(0);
  }

  return 0;
}
