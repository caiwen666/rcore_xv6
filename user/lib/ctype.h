/**
 * rcore-xv6 字符分类与转换
 */

#ifndef RCORE_XV6_CTYPE_H
#define RCORE_XV6_CTYPE_H

/**
 * @brief 判断字符是否为字母或数字
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isalnum(int c);

/**
 * @brief 判断字符是否为字母
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isalpha(int c);

/**
 * @brief 判断字符是否为空白（仅空格与制表符）
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isblank(int c);

/**
 * @brief 判断字符是否为控制字符
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int iscntrl(int c);

/**
 * @brief 判断字符是否为十进制数字
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isdigit(int c);

/**
 * @brief 判断字符是否为可打印字符（非空格）
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isgraph(int c);

/**
 * @brief 判断字符是否为小写字母
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int islower(int c);

/**
 * @brief 判断字符是否为可打印字符（含空格）
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isprint(int c);

/**
 * @brief 判断字符是否为标点符号
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int ispunct(int c);

/**
 * @brief 判断字符是否为空白字符
 *
 * 包含空格、制表符、换行、回车、换页与垂直制表符。
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isspace(int c);

/**
 * @brief 判断字符是否为大写字母
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isupper(int c);

/**
 * @brief 判断字符是否为十六进制数字
 *
 * @param c 待检查的字符（含 EOF）
 * @return 是则返回非 0，否则返回 0
 */
int isxdigit(int c);

/**
 * @brief 将大写字母转换为小写，其他字符原样返回
 *
 * @param c 待转换的字符
 * @return 转换后的字符
 */
int tolower(int c);

/**
 * @brief 将小写字母转换为大写，其他字符原样返回
 *
 * @param c 待转换的字符
 * @return 转换后的字符
 */
int toupper(int c);

#endif
