TOOLPREFIX ?= riscv64-unknown-elf-
CC      := $(TOOLPREFIX)gcc
LD      := $(TOOLPREFIX)ld
OBJCOPY := $(TOOLPREFIX)objcopy
READELF := $(TOOLPREFIX)readelf

APP      ?= hello
TARGET   := target
CRT_DIR  := lib/crt

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
           -nostdlib -nostdinc -ffreestanding -static -Wall -Wextra
LDFLAGS := -T $(CRT_DIR)/linker.ld -nostdlib -static -Wl,--gc-sections

# lib/ 下递归收集全部 .c（排除 crt 启动代码）→ 合并为 lib.o
LIB_SRCS := $(shell find lib -type f -name '*.c' ! -path '$(CRT_DIR)/*')
LIB_OBJS := $(patsubst lib/%.c,$(TARGET)/lib/%.o,$(LIB_SRCS))
LIB_O    := $(TARGET)/lib.o

# 应用源码：APP 为目录时编译该目录下全部 .c，否则编译 $(APP).c
ifneq ($(wildcard $(APP)/*.c),)
APP_SRCS := $(sort $(wildcard $(APP)/*.c))
APP_OBJS := $(addprefix $(TARGET)/,$(notdir $(APP_SRCS:.c=.o)))
else
APP_SRCS := $(APP).c
APP_OBJS := $(TARGET)/$(APP).o
endif

CRT0_O   := $(TARGET)/crt0.o
APP_BIN  := $(TARGET)/$(APP)

.PHONY: all clean lib

all: $(APP_BIN)

lib: $(LIB_O)

$(APP_BIN): $(APP_OBJS) $(LIB_O) $(CRT0_O) | $(TARGET)
	$(CC) $(LDFLAGS) -o $@ $^

$(LIB_O): $(LIB_OBJS) | $(TARGET)
	$(LD) -r -o $@ $^

$(TARGET)/lib/%.o: lib/%.c | $(TARGET)
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c -o $@ $<

$(TARGET)/%.o: %.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(TARGET)/%.o: $(APP)/%.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(CRT0_O): $(CRT_DIR)/crt0.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(TARGET):
	mkdir -p $@

clean:
	rm -rf $(TARGET)
