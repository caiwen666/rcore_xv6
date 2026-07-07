#ifndef SH_DEBUG_H
#define SH_DEBUG_H

#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"
#include "cmd.h"

static const char *cmd_type_name(int type) {
  switch (type) {
  case EXEC:
    return "EXEC";
  case REDIR:
    return "REDIR";
  case PIPE:
    return "PIPE";
  case LIST:
    return "LIST";
  case BACK:
    return "BACK";
  default:
    return "UNKNOWN";
  }
}

static const char *redir_desc(struct redircmd *rcmd) {
  if (rcmd->fd == 0)
    return "stdin  <";
  return "stdout >";
}

static void print_tree_indent(int depth, int is_last) {
  for (int i = 0; i < depth; i++)
    printf(i == depth - 1 ? (is_last ? "   " : "│  ") : "│  ");
}

static void print_cmd(struct cmd *cmd, int depth, int is_last);

static void print_cmd(struct cmd *cmd, int depth, int is_last) {
  print_tree_indent(depth, is_last);
  printf("%s", is_last ? "└─ " : "├─ ");

  if (cmd == NULL) {
    printf(ANSI_YELLOW "(null)" ANSI_RESET "\n");
    return;
  }

  printf(ANSI_BOLD ANSI_GREEN "%s" ANSI_RESET, cmd_type_name(cmd->type));

  switch (cmd->type) {
  case EXEC: {
    struct execcmd *ecmd = (struct execcmd *)cmd;
    int argc = 0;

    while (ecmd->argv[argc] != NULL)
      argc++;
    if (argc == 0) {
      printf(" " ANSI_YELLOW "(empty)" ANSI_RESET "\n");
      break;
    }
    printf("\n");
    for (int i = 0; i < argc; i++) {
      print_tree_indent(depth + 1, i == argc - 1);
      printf("%sargv[%d]: " ANSI_BLUE "\"%s\"" ANSI_RESET "\n",
             i == argc - 1 ? "└─ " : "├─ ", i, ecmd->argv[i]);
    }
    break;
  }

  case REDIR: {
    struct redircmd *rcmd = (struct redircmd *)cmd;
    printf("  " ANSI_YELLOW "%s" ANSI_RESET "  fd=%d  mode=%d  file=" ANSI_BLUE
           "\"%s\"" ANSI_RESET "\n",
           redir_desc(rcmd), rcmd->fd, rcmd->mode, rcmd->file);
    print_cmd(rcmd->cmd, depth + 1, 1);
    break;
  }

  case PIPE: {
    struct pipecmd *pcmd = (struct pipecmd *)cmd;
    printf("\n");
    print_cmd(pcmd->left, depth + 1, 0);
    print_cmd(pcmd->right, depth + 1, 1);
    break;
  }

  case LIST: {
    struct listcmd *lcmd = (struct listcmd *)cmd;
    printf("\n");
    print_cmd(lcmd->left, depth + 1, 0);
    print_cmd(lcmd->right, depth + 1, 1);
    break;
  }

  case BACK: {
    struct backcmd *bcmd = (struct backcmd *)cmd;
    printf("  " ANSI_YELLOW "&" ANSI_RESET "\n");
    print_cmd(bcmd->cmd, depth + 1, 1);
    break;
  }

  default:
    printf("  " ANSI_RED "type=%d" ANSI_RESET "\n", cmd->type);
    break;
  }
}

static void printcmd(struct cmd *cmd) {
  printf(ANSI_BOLD "\n── parsed command ─────────────────────\n" ANSI_RESET);
  if (cmd == NULL)
    printf("  " ANSI_YELLOW "(parse failed)" ANSI_RESET "\n");
  else
    print_cmd(cmd, 0, 1);
  printf(ANSI_BOLD "────────────────────────────────────────\n\n" ANSI_RESET);
}

#endif