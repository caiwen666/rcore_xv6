#include "lineedit.h"
#include "lib/stdio.h"
#include "lib/stdlib.h"
#include "lib/string.h"
#include "common.h"

static char history[MAXHISTORY][MAXLINE];
static int history_count;
static int history_browse = -1;
static char history_draft[MAXLINE];
static int history_draft_valid;

enum {
  KEY_BACKSPACE = 256,
  KEY_DELETE,
  KEY_UP,
  KEY_DOWN,
  KEY_LEFT,
  KEY_RIGHT,
  KEY_HOME,
  KEY_END,
};

typedef struct {
  char data[MAXLINE];
  int len;
  int pos;
} lineedit_t;

static void print_prompt(void) { printf(ANSI_BLUE "%s" ANSI_RESET "$ ", cwd); }

static void refresh_line(lineedit_t *le) {
  printf("\r");
  print_prompt();
  printf("%s", le->data);
  printf("\033[K");
  int tail = le->len - le->pos;
  if (tail > 0)
    printf("\033[%dD", tail);
}

static void history_add(const char *line) {
  if (line[0] == 0)
    return;
  if (history_count > 0 && strcmp(history[history_count - 1], line) == 0)
    return;
  if (history_count < MAXHISTORY) {
    strcpy(history[history_count], line);
    history_count++;
  } else {
    memmove(history[0], history[1], (MAXHISTORY - 1) * MAXLINE);
    strcpy(history[MAXHISTORY - 1], line);
  }
}

static void line_set(lineedit_t *le, const char *s) {
  le->len = strlen(s);
  if (le->len >= MAXLINE)
    le->len = MAXLINE - 1;
  memcpy(le->data, s, le->len);
  le->data[le->len] = 0;
  le->pos = le->len;
  refresh_line(le);
}

// 初始化 lineedit_t 结构体
static void line_clear(lineedit_t *le) {
  le->len = 0;
  le->pos = 0;
  le->data[0] = 0;
}

static void line_insert(lineedit_t *le, char c) {
  if (le->len + 1 >= MAXLINE)
    return;

  if (le->pos == le->len) {
    le->data[le->len++] = c;
    le->data[le->len] = 0;
    le->pos++;
    putchar(STDOUT, c);
    return;
  }

  memmove(le->data + le->pos + 1, le->data + le->pos, le->len - le->pos + 1);
  le->data[le->pos++] = c;
  le->len++;
  refresh_line(le);
}

static void line_backspace(lineedit_t *le) {
  if (le->pos == 0)
    return;

  if (le->pos == le->len) {
    le->len--;
    le->pos--;
    le->data[le->len] = 0;
    printf("\b \b");
    return;
  }

  memmove(le->data + le->pos - 1, le->data + le->pos, le->len - le->pos);
  le->pos--;
  le->len--;
  le->data[le->len] = 0;
  refresh_line(le);
}

static void line_delete(lineedit_t *le) {
  if (le->pos >= le->len)
    return;

  memmove(le->data + le->pos, le->data + le->pos + 1, le->len - le->pos);
  le->len--;
  le->data[le->len] = 0;
  refresh_line(le);
}

static void line_move_left(lineedit_t *le) {
  if (le->pos == 0)
    return;
  le->pos--;
  printf("\033[D");
}

static void line_move_right(lineedit_t *le) {
  if (le->pos >= le->len)
    return;
  le->pos++;
  printf("\033[C");
}

static void line_move_home(lineedit_t *le) {
  if (le->pos == 0)
    return;
  printf("\033[%dD", le->pos);
  le->pos = 0;
}

static void line_move_end(lineedit_t *le) {
  if (le->pos >= le->len)
    return;
  printf("\033[%dC", le->len - le->pos);
  le->pos = le->len;
}

static void line_kill(lineedit_t *le) {
  line_clear(le);
  refresh_line(le);
}

static void history_up(lineedit_t *le) {
  if (history_count == 0)
    return;

  if (history_browse < 0) {
    strcpy(history_draft, le->data);
    history_draft_valid = 1;
    history_browse = history_count - 1;
  } else if (history_browse > 0) {
    history_browse--;
  }

  line_set(le, history[history_browse]);
}

static void history_down(lineedit_t *le) {
  if (history_browse < 0)
    return;

  if (history_browse < history_count - 1) {
    history_browse++;
    line_set(le, history[history_browse]);
    return;
  }

  history_browse = -1;
  if (history_draft_valid)
    line_set(le, history_draft);
  else
    line_set(le, "");
}

static int read_key(void) {
  int c = getchar();
  // 当前为非 ICANON 模式，所以不可能返回 EOF
  if (c == EOF)
    return EOF;

  // 统一换行：部分终端发 \r，内部一律当 \n 处理
  if (c == '\r')
    return '\n';

  // 退格：DEL(0x7f) 与 BS(0x08) 常见于不同终端/键盘
  if (c == 0x7f || c == 0x08)
    return KEY_BACKSPACE;

  // 非 ESC：普通单字节键，直接返回（字母、数字、Ctrl 组合等）
  if (c != 0x1b)
    return c;

  // 以下解析 ANSI 转义序列，常见形式：ESC [ X  或  ESC [ n ~
  // 方向键、Home/End/Delete 等会先发 0x1b，再跟后续字节
  int c2 = getchar();
  if (c2 == EOF)
    return EOF;
  // CSI 序列以 ESC '[' 开头；否则把 ESC 当普通键（极少见）
  if (c2 != '[')
    return c;

  int c3 = getchar();
  if (c3 == EOF)
    return EOF;

  switch (c3) {
  // 方向键：ESC [ A/B/C/D
  case 'A':
    return KEY_UP;
  case 'B':
    return KEY_DOWN;
  case 'C':
    return KEY_RIGHT;
  case 'D':
    return KEY_LEFT;
  // 部分终端：ESC [ H / ESC [ F
  case 'H':
    return KEY_HOME;
  case 'F':
    return KEY_END;
  // 数字区序列：ESC [ 1 ~ / 7 ~ 表示 Home（因终端而异）
  case '1':
  case '7':
    if (getchar() == '~')
      return KEY_HOME;
    return c; // 不完整序列，退回 ESC
  // ESC [ 3 ~ 表示 Delete
  case '3':
    if (getchar() == '~')
      return KEY_DELETE;
    return c; // 不完整序列，退回 ESC
  // ESC [ 4 ~ / 8 ~ 表示 End
  case '4':
  case '8':
    if (getchar() == '~')
      return KEY_END;
    return c; // 不完整序列，退回 ESC
  default:
    return c; // 未识别的 ESC [ ?，退回 ESC
  }
}

static int is_printable(int c) { return c >= 0x20 && c < 0x7f; }

int getcmd(char *buf, int max) {
  lineedit_t le;
  int key;
  int limit = max < MAXLINE ? max : MAXLINE;

  line_clear(&le);
  history_browse = -1;
  history_draft_valid = 0;

  print_prompt();

  while (true) {
    key = read_key();
    if (key == EOF)
      return -1;

    switch (key) {
    case '\n':
      putchar(STDOUT, '\n');
      if (le.len >= limit)
        le.len = limit - 1;
      memcpy(buf, le.data, le.len);
      buf[le.len] = 0;
      history_add(buf);
      history_browse = -1;
      history_draft_valid = 0;
      return 0;

    case 0x04: // Ctrl+D
      if (le.len == 0) {
        putchar(STDOUT, '\n');
        return -1;
      }
      putchar(STDOUT, '\n');
      if (le.len >= limit)
        le.len = limit - 1;
      memcpy(buf, le.data, le.len);
      buf[le.len] = 0;
      return 0;

    case 0x03: // Ctrl+C
      printf("^C\n");
      line_clear(&le);
      history_browse = -1;
      history_draft_valid = 0;
      print_prompt();
      continue;

    case 0x15: // Ctrl+U
      line_kill(&le);
      continue;

    case KEY_BACKSPACE:
      line_backspace(&le);
      continue;

    case KEY_DELETE:
      line_delete(&le);
      continue;

    case KEY_LEFT:
      line_move_left(&le);
      continue;

    case KEY_RIGHT:
      line_move_right(&le);
      continue;

    case KEY_HOME:
      line_move_home(&le);
      continue;

    case KEY_END:
      line_move_end(&le);
      continue;

    case KEY_UP:
      history_up(&le);
      continue;

    case KEY_DOWN:
      history_down(&le);
      continue;

    default:
      if (is_printable(key))
        line_insert(&le, (char)key);
      continue;
    }
  }
}
