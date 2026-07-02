/**
 * rcore-xv6 字符串与内存操作
 */

#ifndef RCORE_XV6_STRING_H
#define RCORE_XV6_STRING_H

#include "types.h"

/**
 * @brief 将内存区域的前 n 个字节设置为指定值
 *
 * 从 dst 指向的地址开始，连续写入 n 个字节，每个字节的值为 c 的低 8 位。
 *
 * @param dst 目标内存区域的起始地址
 * @param c   要填充的字节值（仅低 8 位有效）
 * @param n   要填充的字节数
 * @return 返回 dst
 */
void *memset(void *dst, int c, u32 n);

/**
 * @brief 比较两块内存的前 n 个字节
 *
 * 按无符号字节逐位比较 v1 与 v2 指向的内存，最多比较 n 个字节。
 *
 * @param v1 第一块内存的起始地址
 * @param v2 第二块内存的起始地址
 * @param n  最多比较的字节数
 * @return 若全部相等返回 0；若出现第一个不相等的字节，返回其差值
 *         （以 unsigned char 解释后的差值，即 *s1 - *s2）
 */
int memcmp(const void *v1, const void *v2, u32 n);

/**
 * @brief 将 src 指向的 n 个字节复制到 dst
 *
 * 与 memcpy 不同，本函数正确处理源与目标内存区域重叠的情况：
 * 若源区间与目标区间有重叠且源地址小于目标地址，则从尾部向前复制；
 * 否则从头部向后复制。
 *
 * @param dst 目标内存区域的起始地址
 * @param src 源内存区域的起始地址
 * @param n   要复制的字节数
 * @return 返回 dst
 */
void *memmove(void *dst, const void *src, u32 n);

/**
 * @brief 将 src 指向的 n 个字节复制到 dst
 *
 * 本实现内部调用 memmove，因此在内存重叠时也是安全的。
 * 若明确不存在重叠，语义上与标准 memcpy 相同。
 *
 * @param dst 目标内存区域的起始地址
 * @param src 源内存区域的起始地址
 * @param n   要复制的字节数
 * @return 返回 dst
 */
void *memcpy(void *dst, const void *src, u32 n);

/**
 * @brief 比较两个以 '\0' 结尾的字符串
 *
 * 按无符号字符逐位比较 p 与 q，直到遇到 '\0' 或发现不相等的字符。
 *
 * @param p 第一个以 '\0' 结尾的字符串
 * @param q 第二个以 '\0' 结尾的字符串
 * @return 若两字符串相等返回 0；否则返回第一个不相等字符的差值
 *         （以 unsigned char 解释）
 */
int strcmp(const char *p, const char *q);

/**
 * @brief 比较两个字符串的前 n 个字符
 *
 * 按无符号字符逐位比较 p 与 q，最多比较 n 个字符。
 * 若在某个字符处不相等，或已比较完 n 个字符，或遇到 '\0'，则停止比较。
 *
 * @param p 第一个以 '\0' 结尾的字符串
 * @param q 第二个以 '\0' 结尾的字符串
 * @param n 最多比较的字符数
 * @return 若前 n 个字符（或至 '\0' 为止）全部相等返回 0；
 *         否则返回第一个不相等字符的差值（以 unsigned char 解释）
 */
int strncmp(const char *p, const char *q, u32 n);

/**
 * @brief 将 t 指向的字符串（含 '\0'）复制到 s
 *
 * 从 t 向 s 逐字符复制，直到复制完 t 的结尾 '\0' 为止。
 * 调用方需保证 s 有足够的空间容纳 t 及其结尾的 '\0'。
 *
 * @param s 目标缓冲区
 * @param t 源字符串
 * @return 返回 s
 */
char *strcpy(char *s, const char *t);

/**
 * @brief 从 t 复制最多 n 个字符到 s
 *
 * 从 t 向 s 复制字符，最多复制 n 个。若 t 在复制满 n 个字符前遇到 '\0'，
 * 则剩余的目标空间会用 '\0' 填充。
 *
 * 注意：若 t 的长度大于等于 n 且前 n 个字符均非 '\0'，则 s 可能不以 '\0'
 * 结尾。需要保证 NUL 终止时请使用 safestrcpy。
 *
 * @param s 目标缓冲区
 * @param t 源字符串
 * @param n 最多写入的字符数（含可能追加的 '\0' 填充）
 * @return 返回 s
 */
char *strncpy(char *s, const char *t, int n);

/**
 * @brief 从 t 安全地复制字符串到 s
 *
 * 行为类似 strncpy，但保证 s 始终以 '\0' 结尾：
 * 最多向 s 写入 n - 1 个来自 t 的字符，并在末尾添加 '\0'。
 * 若 n <= 0，则不进行任何写入，直接返回 s。
 *
 * @param s 目标缓冲区
 * @param t 源字符串
 * @param n 目标缓冲区大小（含结尾的 '\0'）
 * @return 返回 s
 */
char *safestrcpy(char *s, const char *t, int n);

/**
 * @brief 计算以 '\0' 结尾的字符串长度
 *
 * 不包含结尾的 '\0'，即返回 s 中 '\0' 之前字符的个数。
 *
 * @param s 以 '\0' 结尾的字符串
 * @return 字符串长度（字节数，不含 '\0'）
 */
int strlen(const char *s);

/**
 * @brief 在字符串中查找指定字符的首次出现位置
 *
 * 在 s 中自左向右扫描，查找与 c 相等的字符（按 char 比较）。
 *
 * @param s 以 '\0' 结尾的字符串
 * @param c 要查找的字符
 * @return 若找到则返回该字符在 s 中的指针；若未找到则返回 0（NULL）
 */
char *strchr(const char *s, char c);

/**
 * @brief 返回错误码对应的描述字符串
 *
 * @param errnum POSIX errno 值
 * @return 指向静态错误描述字符串的指针
 */
char *strerror(int errnum);

#endif
