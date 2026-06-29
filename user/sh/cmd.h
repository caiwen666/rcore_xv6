#ifndef SH_CMD_H
#define SH_CMD_H

#include "common.h"

// Parsed command representation
#define EXEC 1
#define REDIR 2
#define PIPE 3
#define LIST 4
#define BACK 5

struct cmd {
  int type;
};

struct execcmd {
  int type;
  char *argv[MAXARGS];
  char *eargv[MAXARGS];
};

struct redircmd {
  int type;
  struct cmd *cmd;
  char *file;
  char *efile;
  int mode;
  int fd;
};

struct pipecmd {
  int type;
  struct cmd *left;
  struct cmd *right;
};

struct listcmd {
  int type;
  struct cmd *left;
  struct cmd *right;
};

struct backcmd {
  int type;
  struct cmd *cmd;
};

struct cmd *execcmd(void);
struct cmd *redircmd(struct cmd *subcmd, char *file, char *efile, int mode,
                     int fd);
struct cmd *pipecmd(struct cmd *left, struct cmd *right);
struct cmd *listcmd(struct cmd *left, struct cmd *right);
struct cmd *backcmd(struct cmd *subcmd);

void runcmd(struct cmd *cmd);

#endif