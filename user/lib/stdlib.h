/**
 * rcore-xv6 标准库工具函数
 */

#ifndef RCORE_XV6_STDLIB_H
#define RCORE_XV6_STDLIB_H

#include "types.h"
#include "errno.h"

#define NULL 0
#define true 1
#define false 0

/**
 * @brief 将字符串转换为整数
 *
 * 从 s 的起始位置开始，连续读取数字字符 '0'～'9' 并累加为十进制整数。
 * 遇到第一个非数字字符时停止解析。
 *
 * 注意：本实现不处理前导空白、正负号或溢出；s 为空或非数字开头时返回 0。
 *
 * @param s 以数字字符开头的字符串
 * @return 解析得到的整数值
 */
int atoi(const char *s);

/**
 * @brief 在堆上分配 nbytes 字节的内存
 *
 * 采用 K&R 风格的首次适配（first-fit）空闲链表分配器：
 * 在已有空闲块中查找足够大的块；若无合适块，则通过 sbrk 向内核申请更多堆空间
 * （单次至少扩展 4096 个 Header 单位），再从中划分出所需大小。
 *
 * 返回的指针指向用户可用区域；分配器在返回地址之前维护一块 Header 元数据，
 * 用户不应访问该头部。
 *
 * 注意：
 * - 分配失败（例如 sbrk 返回 -1）时返回 NULL（0）
 * - 未对 nbytes 为 0 做特殊处理
 * - 本实现不是线程安全的
 *
 * @param nbytes 需要分配的字节数
 * @return 成功时返回指向已分配内存的指针；失败时返回 NULL
 */
void *malloc(u32 nbytes);

/**
 * @brief 释放先前由 malloc 分配的内存
 *
 * 将 ap 指向的用户块归还到空闲链表，并尝试与相邻空闲块合并（coalesce），
 * 以减少堆碎片。ap 必须是 malloc 返回的指针；释放后该指针不应再被使用。
 *
 * 注意：
 * - 对 NULL 指针调用、重复释放（double free）或未由 malloc 分配的指针调用，
 *   会导致未定义行为
 * - 本实现不是线程安全的
 *
 * @param ap 待释放的内存指针（由 malloc 返回）
 */
void free(void *ap);

#endif
