TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/riscv64gc-unknown-none-elf/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
KERNEL_QEMU := ../bin/kernel-rvqemu
FS_IMG := ../user/target/$(TARGET)/$(MODE)/fs.img

BOARD := rvqemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64


build: $(KERNEL_BIN) mv fs-img

mv:
	@cp $(KERNEL_BIN) ${KERNEL_QEMU}

$(KERNEL_BIN): kernel
	@$(OBJCOPY) ${KERNEL_ELF} --strip-all -O binary $@

kernel: pre user
	@echo Platform: $(BOARD), SBI: $(SBI)
	@cp src/hal/arch/riscv/linker-$(BOARD).ld src/hal/arch/riscv/linker.ld
	@LOG=${LOG} cargo build --${MODE} --target $(TARGET) --features "board_$(BOARD)"

pre:
	@rm .cargo/config.toml || true
	@cp cargo/rv-config.toml .cargo/config.toml

fs-img:
	@cd ../easy-fs-fuse && cargo run --release -- -s ../user/src/bin -t ../user/target/$(TARGET)/$(MODE)/

user:
	@cd ../user && make build

run:
	qemu-system-riscv64 \
	-machine virt \
	-kernel $(KERNEL_QEMU) \
	-m 128M \
	-nographic \
	-smp 2	\
	-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

#	-bios $(BOOTLOADER) \
#	-drive file=sdcard.img,if=none,format=raw,id=x0  \
#	-device virtio-net-device,netdev=net \
#	-netdev user,id=net \
#	-initrd initrd.img


clean:
	@rm src/hal/arch/riscv/linker.ld