# 内核目录
KERNEL = kernel
# 用户程序目录
USER = user
# 产物存放的目录
OUTPUT = target
# ext2 镜像
EXT2_IMG = ext2.img
IMAGE_DIR = image
EXT2_SIZE_MB = 1024

# 输出目录不存在就创建
$(shell mkdir -p $(OUTPUT))

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
QEMU_FLAGS += -drive file=$(EXT2_IMG),if=none,format=raw,id=hd0
QEMU_FLAGS += -device virtio-blk-device,drive=hd0,bus=virtio-mmio-bus.0

.PHONY: build run debug gdb user image

user:
	$(MAKE) -C $(USER)

image: user
	IMAGE_DIR=$(IMAGE_DIR) bash scripts/prepare_image.sh
	rm -f $(EXT2_IMG)
	dd if=/dev/zero of=$(EXT2_IMG) bs=1M count=$(EXT2_SIZE_MB) status=none
	mke2fs -t ext2 -F -I 128 -d $(IMAGE_DIR) $(EXT2_IMG)

build:
	cd $(KERNEL) && $(CARGO) build $(CARGO_BUILD_FLAGS)
	$(RUST_OBJCOPY) $(RUST_OBJCOPY_FLAGS)

run: build
	@test -f $(EXT2_IMG) || $(MAKE) image
	$(QEMU) $(QEMU_FLAGS)

debug: build
	@test -f $(EXT2_IMG) || $(MAKE) image
	$(QEMU) $(QEMU_FLAGS) -s -S

gdb:
	$(GDB) \
		-ex 'file $(OUTPUT)/kernel.elf' \
		-ex 'set arch riscv:rv64' \
		-ex 'target remote localhost:1234'