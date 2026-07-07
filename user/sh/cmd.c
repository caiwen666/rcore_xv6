#include "cmd.h"
#include "lib/stdlib.h"
#include "lib/string.h"

struct cmd *execcmd(void) {
  struct execcmd *cmd;

  cmd = malloc(sizeof(*cmd));
  memset(cmd, 0, sizeof(*cmd));
  cmd->type = EXEC;
  return (struct cmd *)cmd;
}

struct cmd *redircmd(struct cmd *subcmd, char *file, char *efile, int mode,
                     int fd) {
  struct redircmd *cmd;

  cmd = malloc(sizeof(*cmd));
  memset(cmd, 0, sizeof(*cmd));
  cmd->type = REDIR;
  cmd->cmd = subcmd;
  cmd->file = file;
  cmd->efile = efile;
  cmd->mode = mode;
  cmd->fd = fd;
  return (struct cmd *)cmd;
}

struct cmd *pipecmd(struct cmd *left, struct cmd *right) {
  struct pipecmd *cmd;

  cmd = malloc(sizeof(*cmd));
  memset(cmd, 0, sizeof(*cmd));
  cmd->type = PIPE;
  cmd->left = left;
  cmd->right = right;
  return (struct cmd *)cmd;
}

struct cmd *listcmd(struct cmd *left, struct cmd *right) {
  struct listcmd *cmd;

  cmd = malloc(sizeof(*cmd));
  memset(cmd, 0, sizeof(*cmd));
  cmd->type = LIST;
  cmd->left = left;
  cmd->right = right;
  return (struct cmd *)cmd;
}

struct cmd *backcmd(struct cmd *subcmd) {
  struct backcmd *cmd;

  cmd = malloc(sizeof(*cmd));
  memset(cmd, 0, sizeof(*cmd));
  cmd->type = BACK;
  cmd->cmd = subcmd;
  return (struct cmd *)cmd;
}

// Execute cmd.  Never returns.
// TODO
// void runcmd(struct cmd *cmd) {
//   int p[2];
//   struct backcmd *bcmd;
//   struct execcmd *ecmd;
//   struct listcmd *lcmd;
//   struct pipecmd *pcmd;
//   struct redircmd *rcmd;

//   if (cmd == 0)
//     exit(1);

//   switch (cmd->type) {
//   default:
//     panic("runcmd");

//   case EXEC:
//     ecmd = (struct execcmd *)cmd;
//     if (ecmd->argv[0] == 0)
//       exit(1);
//     exec(ecmd->argv[0], ecmd->argv);
//     fprintf(2, "exec %s failed\n", ecmd->argv[0]);
//     break;

//   case REDIR:
//     rcmd = (struct redircmd *)cmd;
//     close(rcmd->fd);
//     if (open(rcmd->file, rcmd->mode) < 0) {
//       fprintf(2, "open %s failed\n", rcmd->file);
//       exit(1);
//     }
//     runcmd(rcmd->cmd);
//     break;

//   case LIST:
//     lcmd = (struct listcmd *)cmd;
//     if (fork1() == 0)
//       runcmd(lcmd->left);
//     wait(0);
//     runcmd(lcmd->right);
//     break;

//   case PIPE:
//     pcmd = (struct pipecmd *)cmd;
//     if (pipe(p) < 0)
//       panic("pipe");
//     if (fork1() == 0) {
//       close(1);
//       dup(p[1]);
//       close(p[0]);
//       close(p[1]);
//       runcmd(pcmd->left);
//     }
//     if (fork1() == 0) {
//       close(0);
//       dup(p[0]);
//       close(p[0]);
//       close(p[1]);
//       runcmd(pcmd->right);
//     }
//     close(p[0]);
//     close(p[1]);
//     wait(0);
//     wait(0);
//     break;

//   case BACK:
//     bcmd = (struct backcmd *)cmd;
//     if (fork1() == 0)
//       runcmd(bcmd->cmd);
//     break;
//   }
//   exit(0);
// }