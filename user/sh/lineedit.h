#ifndef SH_LINEEDIT_H
#define SH_LINEEDIT_H

/**
 * @brief 读取一行经行编辑后的命令
 *
 * 在 raw 模式下自行处理回显、光标移动、历史浏览等。
 * 成功时将结果写入 buf（不含换行），返回 0。
 * 空行 Ctrl+D 返回 -1 表示 EOF。
 *
 * @param buf 输出缓冲区
 * @param max buf 总大小（含结尾 '\0'）
 * @return 0 成功，-1 EOF
 */
int getcmd(char *buf, int max);

#endif
