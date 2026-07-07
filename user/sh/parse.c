#include "parse.h"
#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"
#include "cmd.h"
#include "common.h"

static struct cmd *parseline(char **ps, char *es);
static struct cmd *parsepipe(char **ps, char *es);
static struct cmd *parseredirs(struct cmd *cmd, char **ps, char *es);
static struct cmd *parseblock(char **ps, char *es);
static struct cmd *parseexec(char **ps, char *es);
static struct cmd *nulterminate(struct cmd *cmd);
static int gettoken(char **ps, char *es, char **q, char **eq);
static int peek(char **ps, char *es, char *toks);

static const char whitespace[] = " \t\r\n\v";
static const char symbols[] = "<|>&;()";

static struct cmd *parse_fail(const char *msg) {
  fprintf(STDERR, "sh: syntax error: %s\n", msg);
  return NULL;
}

struct cmd *parsecmd(char *s) {
  char *es;
  struct cmd *cmd;

  es = s + strlen(s);
  cmd = parseline(&s, es);
  if (cmd == NULL)
    return NULL;
  peek(&s, es, "");
  if (s != es) {
    fprintf(STDERR, "sh: syntax error: unexpected input near '%s'\n", s);
    return NULL;
  }
  return nulterminate(cmd);
}

static struct cmd *parseline(char **ps, char *es) {
  struct cmd *cmd;
  struct cmd *next;

  cmd = parsepipe(ps, es);
  if (cmd == NULL)
    return NULL;
  while (peek(ps, es, "&")) {
    gettoken(ps, es, 0, 0);
    cmd = backcmd(cmd);
  }
  if (peek(ps, es, ";")) {
    gettoken(ps, es, 0, 0);
    next = parseline(ps, es);
    if (next == NULL)
      return NULL;
    cmd = listcmd(cmd, next);
  }
  return cmd;
}

static struct cmd *parsepipe(char **ps, char *es) {
  struct cmd *cmd;
  struct cmd *right;

  cmd = parseexec(ps, es);
  if (cmd == NULL)
    return NULL;
  if (peek(ps, es, "|")) {
    gettoken(ps, es, 0, 0);
    right = parsepipe(ps, es);
    if (right == NULL)
      return NULL;
    cmd = pipecmd(cmd, right);
  }
  return cmd;
}

static struct cmd *parseredirs(struct cmd *cmd, char **ps, char *es) {
  int tok;
  char *q, *eq;

  while (peek(ps, es, "<>")) {
    tok = gettoken(ps, es, 0, 0);
    if (gettoken(ps, es, &q, &eq) != 'a')
      return parse_fail("missing filename for redirection");
    // TODO 添加文件打开模式
    switch (tok) {
    case '<':
      cmd = redircmd(cmd, q, eq, 0, 0);
      break;
    case '>':
      cmd = redircmd(cmd, q, eq, 0, 1);
      break;
    case '+': // >>
      cmd = redircmd(cmd, q, eq, 0, 1);
      break;
    }
  }
  return cmd;
}

static struct cmd *parseblock(char **ps, char *es) {
  struct cmd *cmd;

  if (!peek(ps, es, "("))
    return parse_fail("expected '('");
  gettoken(ps, es, 0, 0);
  cmd = parseline(ps, es);
  if (cmd == NULL)
    return NULL;
  if (!peek(ps, es, ")"))
    return parse_fail("expected ')'");
  gettoken(ps, es, 0, 0);
  return parseredirs(cmd, ps, es);
}

static struct cmd *parseexec(char **ps, char *es) {
  char *q, *eq;
  int tok, argc;
  struct execcmd *cmd;
  struct cmd *ret;

  if (peek(ps, es, "("))
    return parseblock(ps, es);

  ret = execcmd();
  cmd = (struct execcmd *)ret;

  argc = 0;
  ret = parseredirs(ret, ps, es);
  if (ret == NULL)
    return NULL;
  while (!peek(ps, es, "|)&;")) {
    if ((tok = gettoken(ps, es, &q, &eq)) == 0)
      break;
    if (tok != 'a')
      return parse_fail("expected command argument");
    if (argc >= MAXARGS)
      return parse_fail("too many arguments");
    cmd->argv[argc] = q;
    cmd->eargv[argc] = eq;
    argc++;
    ret = parseredirs(ret, ps, es);
    if (ret == NULL)
      return NULL;
  }
  cmd->argv[argc] = 0;
  cmd->eargv[argc] = 0;
  return ret;
}

static int gettoken(char **ps, char *es, char **q, char **eq) {
  char *s;
  int ret;

  s = *ps;
  while (s < es && strchr(whitespace, *s))
    s++;
  if (q)
    *q = s;
  ret = *s;
  switch (*s) {
  case 0:
    break;
  case '|':
  case '(':
  case ')':
  case ';':
  case '&':
  case '<':
    s++;
    break;
  case '>':
    s++;
    if (*s == '>') {
      ret = '+';
      s++;
    }
    break;
  default:
    ret = 'a';
    while (s < es && !strchr(whitespace, *s) && !strchr(symbols, *s))
      s++;
    break;
  }
  if (eq)
    *eq = s;

  while (s < es && strchr(whitespace, *s))
    s++;
  *ps = s;
  return ret;
}

static int peek(char **ps, char *es, char *toks) {
  char *s;

  s = *ps;
  while (s < es && strchr(whitespace, *s))
    s++;
  *ps = s;
  return *s && strchr(toks, *s);
}

// NUL-terminate all the counted strings.
static struct cmd *nulterminate(struct cmd *cmd) {
  int i;
  struct backcmd *bcmd;
  struct execcmd *ecmd;
  struct listcmd *lcmd;
  struct pipecmd *pcmd;
  struct redircmd *rcmd;

  if (cmd == NULL)
    return NULL;

  switch (cmd->type) {
  case EXEC:
    ecmd = (struct execcmd *)cmd;
    for (i = 0; ecmd->argv[i]; i++)
      *ecmd->eargv[i] = 0;
    break;

  case REDIR:
    rcmd = (struct redircmd *)cmd;
    nulterminate(rcmd->cmd);
    *rcmd->efile = 0;
    break;

  case PIPE:
    pcmd = (struct pipecmd *)cmd;
    nulterminate(pcmd->left);
    nulterminate(pcmd->right);
    break;

  case LIST:
    lcmd = (struct listcmd *)cmd;
    nulterminate(lcmd->left);
    nulterminate(lcmd->right);
    break;

  case BACK:
    bcmd = (struct backcmd *)cmd;
    nulterminate(bcmd->cmd);
    break;
  }
  return cmd;
}
