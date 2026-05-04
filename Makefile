# 内核目录
KERNEL = kernel
# 产物存放的目录
OUTPUT = target

# 工具
QEMU = qemu-system-riscv64
CARGO = cargo
RUST_OBJCOPY = rust-objcopy
GDB = riscv64-unknown-elf-gdb

# 编译模式
BUILD ?= debug
ifeq ($(BUILD), release)
	STRIP ?= 1
else
	STRIP ?= 0
endif

# QEMU 启用 CPU 数量 
CPUS ?= 4

# 准备参数
ifeq ($(BUILD), release)
	CARGO_BUILD_FLAGS += --release
	KERNEL_ELF = $(KERNEL)/target/riscv64gc-unknown-none-elf/release/kernel
else
	KERNEL_ELF = $(KERNEL)/target/riscv64gc-unknown-none-elf/debug/kernel
endif

RUST_OBJCOPY_FLAGS += --binary-architecture=riscv64
ifeq ($(STRIP), 1)
	RUST_OBJCOPY_FLAGS += --strip-all
endif
RUST_OBJCOPY_FLAGS += $(KERNEL_ELF)
RUST_OBJCOPY_FLAGS += $(OUTPUT)/kernel.elf
QEMU_FLAGS += -machine virt
QEMU_FLAGS += -nographic
QEMU_FLAGS += -bios none
QEMU_FLAGS += -kernel $(OUTPUT)/kernel.elf
QEMU_FLAGS += -m 128M
QEMU_FLAGS += -smp $(CPUS)

.PHONY: build run debug gdb

build:
	cd $(KERNEL) && $(CARGO) build $(CARGO_BUILD_FLAGS)
	$(RUST_OBJCOPY) $(RUST_OBJCOPY_FLAGS)

run: build
	$(QEMU) $(QEMU_FLAGS)

debug: build
	$(QEMU) $(QEMU_FLAGS) -s -S

gdb:
	$(GDB) \
		-ex 'file $(OUTPUT)/kernel.elf' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'