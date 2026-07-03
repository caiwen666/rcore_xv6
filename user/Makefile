TOOLPREFIX ?= riscv64-unknown-elf-
CC      := $(TOOLPREFIX)gcc
LD      := $(TOOLPREFIX)ld
OBJCOPY := $(TOOLPREFIX)objcopy
READELF := $(TOOLPREFIX)readelf

# 单个程序：make APP=hello
# 多个程序：make APPS="hello sh"
# 在 Makefile 内写默认值时不要加引号，应写：APPS ?= hello sh
# APP 的名字应该是 user 目录下对应的 .c 文件或是文件夹。
# 对于前者，会编译 .c 文件，对于后者，会将文件夹下的所有文件都编译。
APPS ?= hello sh
ifneq ($(origin APP),undefined)
APPS := $(APP)
endif

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

CRT0_O   := $(TARGET)/crt0.o
APP_BINS := $(addprefix $(TARGET)/,$(APPS))

.PHONY: all clean lib

all: $(APP_BINS)

lib: $(LIB_O)

# 每个 app 单独生成编译与链接规则，对象文件放在 target/obj/<app>/ 下避免与二进制 target/<app> 冲突
define app_rules
ifneq ($$(wildcard $(1)/*.c),)
APP_OBJS_$(1) := $$(addprefix $(TARGET)/obj/$(1)/,$$(notdir $$(patsubst %.c,%.o,$$(sort $$(wildcard $(1)/*.c)))))

$(TARGET)/obj/$(1)/%.o: $(1)/%.c | $(TARGET)
	@mkdir -p $$(dir $$@)
	$(CC) $(CFLAGS) -c -o $$@ $$<
else
APP_OBJS_$(1) := $(TARGET)/obj/$(1)/$(1).o

$(TARGET)/obj/$(1)/$(1).o: $(1).c | $(TARGET)
	@mkdir -p $$(dir $$@)
	$(CC) $(CFLAGS) -c -o $$@ $$<
endif

$(TARGET)/$(1): $$(APP_OBJS_$(1)) $(LIB_O) $(CRT0_O) | $(TARGET)
	$(CC) $(LDFLAGS) -o $$@ $$^
endef

$(foreach app,$(APPS),$(eval $(call app_rules,$(app))))

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
