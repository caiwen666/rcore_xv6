/**
 * rcore-xv6 标准输入输出
 */

#ifndef RCORE_XV6_STDIO_H
#define RCORE_XV6_STDIO_H

#define STDIN 0
#define STDOUT 1
#define STDERR 2

#define ANSI_RESET "\033[0m"
#define ANSI_RED "\033[31m"
#define ANSI_GREEN "\033[32m"
#define ANSI_YELLOW "\033[33m"
#define ANSI_BLUE "\033[34m"
#define ANSI_BOLD "\033[1m"

#define EOF -1

/**
 * @brief 向指定文件描述符写入一个字符
 *
 * 通过 write 系统调用向 fd 写入单字节 c。
 *
 * @param fd 目标文件描述符
 * @param c  要写入的字符
 */
void putchar(int fd, char c);

/**
 * @brief 按格式字符串向标准输出打印
 *
 * 等价于 fprintf(STDOUT, fmt, ...)。
 *
 * 当前支持的格式说明符：
 * - %%  输出字面量 '%'
 * - %c  字符（int 参数，取低 8 位）
 * - %d  有符号十进制整数
 * - %l  无符号 64 位十进制整数
 * - %x  无符号十六进制整数（小写）
 * - %p  指针（以 0x 开头的 64 位十六进制）
 * - %s  以 '\0' 结尾的字符串；NULL 时输出 "(null)"
 *
 * @param fmt 格式字符串
 * @param ... 与格式说明符对应的参数
 */
void printf(const char *fmt, ...);

/**
 * @brief 按格式字符串向指定文件描述符打印
 *
 * 语义与 printf 相同，但输出目标为 fd。
 *
 * @param fd  目标文件描述符
 * @param fmt 格式字符串
 * @param ... 与格式说明符对应的参数
 */
void fprintf(int fd, const char *fmt, ...);

/**
 * @brief 从标准输入读取一个字符
 * @return 如果没有读到字符，则返回 EOF
 */
int getchar(void);

/**
 * @brief 从标准输入读取一行到缓冲区
 *
 * 从指定文件描述符中逐字节读取，写入 buf，直到出现以下任一情况：
 * - 读入 '\n' 或 '\r'
 * - 已写入 max - 1 个字符（为结尾 '\0' 预留一字节）
 * - 遇到了 EOF
 *
 * 读取结束后始终在 buf 末尾写入 '\0'。
 *
 * 注意：不检查缓冲区溢出以外的边界；不丢弃行尾换行符，会保留在 buf 中。
 *
 * @param buf 目标缓冲区
 * @param max 缓冲区总大小（含结尾 '\0'）
 * @return 返回读到的字符数
 */
int getline(int fd, char *buf, int max);

#endif
