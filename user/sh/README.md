# rcore_xv6 Shell

改编自 xv6 的 shell。

## 1. 支持的键盘操作

| 按键 | 作用 |
|------|------|
| 可打印字符 | 插入字符 |
| `Enter` | 提交当前行 |
| `Backspace` / `Delete` | 删除光标前 / 后字符 |
| `←` `→` | 光标左右移动 |
| `Home` / `End` | 跳到行首 / 行尾 |
| `↑` `↓` | 浏览历史（最多 100 条，相邻重复命令不重复记录） |
| `Ctrl+C` | 清空当前行，重新显示提示符 |
| `Ctrl+U` | 清空当前行（kill line） |
| `Ctrl+D` | 若当前行为空则退出 shell；否则提交当前行 |

## 2. 限制

- 单行最长 **100** 字符（`MAXLINE`），超出部分在提交时截断。
- 不支持引号、转义、Tab 补全。
- 历史记录保存在内存中，shell 退出后丢失。

---

## 3. 内建命令

### 3.1 `cd` — 切换工作目录

`cd` 在 `main.c` 中单独处理，**不经过** `parsecmd` 解析器。

**格式：**

```text
cd <路径>
```

要求第三个字符必须是空格（即字面量 `cd ` 前缀）。

**示例：**

```sh
cd /root
cd ..
cd subdir
```

**行为：**

- 成功：静默切换目录，并调用 `getcwd` 更新提示符路径。
- 失败：打印 `sh: cannot cd <路径>`。
- `getcwd` 失败：回退到 `/` 并提示。

**注意：** 下列写法**不会**被当作 `cd` 内建处理，而是交给解析器：

```sh
cd          # 缺少路径
cd..        # 缺少空格
```

---

## 4. 可解析的命令语法

**保留符号：** `<` `>` `|` `&` `;` `(` `)`

**参数上限：** 每条简单命令最多 **10** 个参数（`MAXARGS`）。

---

### 4.1 简单命令（`EXEC`）

运行一个程序名加若干参数。

```sh
ls
echo hello world
/bin/sh arg1 arg2
```

解析为：

```text
EXEC
└─ argv[0]: "ls"
```

---

### 4.2 输入重定向（`REDIR`，stdin）

将标准输入重定向到文件。

**格式：**

```text
< 文件名
```

可出现在命令前或参数之间。

**示例：**

```sh
< input.txt wc
wc < input.txt
sort < data.txt -o out
```

解析为对子命令包一层 `REDIR`（`fd=0`，`stdin <`）。

---

### 4.3 输出重定向（`REDIR`，stdout）

将标准输出重定向到文件。

**格式：**

```text
> 文件名
>> 文件名
```

`>` 与 `>>` 在解析阶段均识别，前者为截断，后者为追加。

**示例：**

```sh
echo hello > out.txt
ls -l >> log.txt
> empty.txt echo
```

---

### 4.4 管道（`PIPE`）

将左侧命令的标准输出连接到右侧命令的标准输入。

**格式：**

```text
命令1 | 命令2 | 命令3 ...
```

管道优先级高于 `;` 和 `&`。

**示例：**

```sh
ls | wc
cat file | grep foo | wc -l
echo hello | cat
```

解析为左结合的 `PIPE` 树：

```text
PIPE
├─ EXEC  ls
└─ EXEC  wc
```

---

### 4.5 顺序执行（`LIST`）

用分号连接多条命令，表示顺序执行（语义上先左后右；执行逻辑在 `runcmd` 中，当前未启用）。

**格式：**

```text
命令1 ; 命令2 ; 命令3
```

**示例：**

```sh
echo first ; echo second
ls ; pwd ; echo done
```

解析为：

```text
LIST
├─ EXEC  echo first
└─ EXEC  echo second
```

---

### 4.6 后台执行（`BACK`）

在命令末尾加 `&`，表示后台运行。

**格式：**

```text
命令 &
```

`&` 可紧跟在管道、重定向或简单命令之后；同一「行片段」末尾可有一个 `&`。

**示例：**

```sh
sleep 100 &
echo hello &
ls | wc &
```

解析为：

```text
BACK  &
└─ EXEC  echo hello
```

---

### 4.7 子 shell / 分组（`(` `)`）

用圆括号将一条或多条命令（可含 `;`、`|`、`&` 等）组合为子结构，括号内再按正常规则解析。

**格式：**

```text
( 命令序列 )
```

括号后可以跟重定向。

**示例：**

```sh
(echo a ; echo b)
(ls ; pwd) > out.txt
(echo foo | cat)
```

解析为先 `LIST` / `PIPE` / `EXEC`，再可选外包 `REDIR`。

---

### 4.8 组合语法

解析器允许将上述结构任意组合（在语法合法的前提下）。

**示例：**

```sh
# 管道 + 后台
ls | grep x &

── parsed command ─────────────────────
└─ BACK  &
   └─ PIPE
│  │  ├─ EXEC
│  │     └─ argv[0]: "ls"
│     └─ EXEC
│  │  │  ├─ argv[0]: "grep"
│  │     └─ argv[1]: "x"
────────────────────────────────────────

# 分组 + 重定向 + 顺序
(echo start ; ls) > log.txt ; echo end

── parsed command ─────────────────────
└─ LIST
│  ├─ REDIR  stdout >  fd=1  mode=0  file="log.txt"
│     └─ LIST
│  │  │  ├─ EXEC
│  │  │  │  ├─ argv[0]: "echo"
│  │  │     └─ argv[1]: "start"
│  │     └─ EXEC
│  │  │     └─ argv[0]: "ls"
   └─ EXEC
│  │  ├─ argv[0]: "echo"
│     └─ argv[1]: "end"
────────────────────────────────────────

# 多重定向（作用于同一简单命令）
wc < in.txt > out.txt

── parsed command ─────────────────────
└─ REDIR  stdout >  fd=1  mode=0  file="out.txt"
   └─ REDIR  stdin  <  fd=0  mode=0  file="in.txt"
│     └─ EXEC
│  │     └─ argv[0]: "wc"
────────────────────────────────────────

# 重定向夹在参数之间
echo hello > out.txt world

── parsed command ─────────────────────
└─ REDIR  stdout >  fd=1  mode=0  file="out.txt"
   └─ EXEC
│  │  ├─ argv[0]: "echo"
│  │  ├─ argv[1]: "hello"
│     └─ argv[2]: "world"
────────────────────────────────────────

# 复杂组合
(cat < a.txt | sort) > b.txt &

── parsed command ─────────────────────
└─ BACK  &
   └─ REDIR  stdout >  fd=1  mode=0  file="b.txt"
│     └─ PIPE
│  │  │  ├─ REDIR  stdin  <  fd=0  mode=0  file="a.txt"
│  │  │     └─ EXEC
│  │  │  │     └─ argv[0]: "cat"
│  │     └─ EXEC
│  │  │     └─ argv[0]: "sort"
────────────────────────────────────────
```

---

## 5. 不支持的特性

与常见 Unix shell 相比，当前**不支持**：

- 引号与转义（`'...'`、`"..."`、`\ `）
- 环境变量展开（`$HOME`、`$?`）
- 通配符（`*`、`?`）
- 注释（`# ...`）
- `&&`、`||`、`|&`
- 内建 `exit`、`export`、`source` 等
- 作业控制（`fg`、`bg`、`jobs`）
- 文件名含空格的正确拆分（未加引号时按空白切分）
