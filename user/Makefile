TOOLPREFIX ?= riscv64-unknown-elf-
CC      := $(TOOLPREFIX)gcc
LD      := $(TOOLPREFIX)ld
OBJCOPY := $(TOOLPREFIX)objcopy
READELF := $(TOOLPREFIX)readelf

# 用户程序列表：路径相对于 user/ 目录
#   .c 文件：编译该文件并链接
#   目录：编译目录下所有 .c 并链接
# 示例：hello.c sh tests/my/fork_sleep.c
# 单个程序：make APP=hello.c
PROGS ?= hello.c sleep.c ls.c sh tests/my/fork_sleep.c tests/my/exec.c
ifneq ($(origin APP),undefined)
PROGS := $(APP)
endif

TARGET  := target
CRT_DIR := lib/crt

# -march=rv64gc: 设置架构，并开启 G（整数 + 乘除 + 原子 + 单/双精度浮点）指令集
# 	C（部分指令压缩到 16 位）指令集
# -mabi=lp64: 指针是 64 位，int 是 32 位
# -mcmodel=medany: 设置代码模型，代码和数据可以在任意 64 位地址访问
# -fno-pie -fno-pic: 不生成位置无关代码
# -fno-stack-protector: 不加金丝雀
# -nostdlib: 不链接标准库
# -nostdinc: 不使用工具链标准头文件
# -ffreestanding: 独立环境，不依赖 hosted libc
# -static: 不链接动态库
# -Wall -Wextra: 启用所有警告
CFLAGS  := -march=rv64gc -mabi=lp64 -mcmodel=medany \
           -fno-pie -fno-pic -fno-stack-protector \
           -nostdlib -nostdinc -ffreestanding -static -Wall -Wextra \
           -I.
LDFLAGS := -T $(CRT_DIR)/linker.ld -nostdlib -static -Wl,--gc-sections

# lib/ 下递归收集全部 .c（排除 crt 启动代码）→ 合并为 lib.o
LIB_SRCS := $(shell find lib -type f -name '*.c' ! -path '$(CRT_DIR)/*')
LIB_OBJS := $(patsubst lib/%.c,$(TARGET)/lib/%.o,$(LIB_SRCS))
LIB_O    := $(TARGET)/lib.o
CRT0_O   := $(TARGET)/crt0.o

# prog_bin:  hello.c -> target/hello, sh -> target/sh
#           tests/my/fork_sleep.c -> target/tests/my/fork_sleep
prog_bin  = $(TARGET)/$(if $(filter %.c,$(1)),$(patsubst %.c,%,$(1)),$(1))
# prog_objs: 对应的对象文件，路径保留在 target/obj/ 下
prog_objs = $(if $(filter %.c,$(1)),\
              $(TARGET)/obj/$(patsubst %.c,%.o,$(1)),\
              $(patsubst $(1)/%.c,$(TARGET)/obj/$(1)/%.o,$(sort $(wildcard $(1)/*.c))))

PROG_BINS := $(foreach p,$(PROGS),$(call prog_bin,$(p)))

.PHONY: all clean lib

all: lib $(PROG_BINS)

lib: $(LIB_O) $(CRT0_O)

define link_rule
$(call prog_bin,$(1)): $(call prog_objs,$(1)) $(LIB_O) $(CRT0_O) | $(TARGET)
	@mkdir -p $$(dir $$@)
	$(CC) $(LDFLAGS) -o $$@ $(call prog_objs,$(1)) $(LIB_O) $(CRT0_O)
endef

$(foreach p,$(PROGS),$(eval $(call link_rule,$(p))))

# 用户程序源文件 → target/obj/<保留路径>.o
# -I. 使 #include "lib/..." 从 user/ 根目录解析
# -I$(dir $<) 使 #include "cmd.h" 等同目录头文件可从任意子目录引用
$(TARGET)/obj/%.o: %.c | $(TARGET)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -I$(dir $<) -c -o $@ $<

$(LIB_O): $(LIB_OBJS) | $(TARGET)
	$(LD) -r -o $@ $^

$(TARGET)/lib/%.o: lib/%.c | $(TARGET)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c -o $@ $<

$(CRT0_O): $(CRT_DIR)/crt0.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(TARGET):
	mkdir -p $@

clean:
	rm -rf $(TARGET)
