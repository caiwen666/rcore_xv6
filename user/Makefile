TOOLPREFIX ?= riscv64-unknown-elf-
CC      := $(TOOLPREFIX)gcc
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
# -static: 不链接动态库
# -Wall -Wextra: 启用所有警告
CFLAGS  := -march=rv64gc -mabi=lp64 -mcmodel=medany \
           -fno-pie -fno-pic -fno-stack-protector \
           -nostdlib -static -Wall -Wextra
LDFLAGS := -T $(CRT_DIR)/linker.ld -nostdlib -static -Wl,--gc-sections

CRT0_O   := $(TARGET)/crt0.o
APP_BIN  := $(TARGET)/$(APP)
APP_O    := $(TARGET)/$(APP).o

.PHONY: all clean

all: $(APP_BIN)

$(APP_BIN): $(APP_O) $(CRT0_O) | $(TARGET)
	$(CC) $(LDFLAGS) -o $@ $^

$(TARGET)/%.o: %.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(CRT0_O): $(CRT_DIR)/crt0.c | $(TARGET)
	$(CC) $(CFLAGS) -c -o $@ $<

$(TARGET):
	mkdir -p $@

clean:
	rm -rf $(TARGET)
