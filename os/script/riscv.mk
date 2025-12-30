TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/riscv64gc-unknown-none-elf/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin

BOARD := rvqemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# KERNEL ENTRY
KERNEL_ENTRY_PA := 0x80200000

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64


build: $(KERNEL_BIN) mv

mv:
	@cp $(KERNEL_BIN) ../kernel-qemu

$(KERNEL_BIN): kernel
	@$(OBJCOPY) ${KERNEL_ELF} --strip-all -O binary $@

kernel:
	@echo Platform: $(BOARD), SBI: $(SBI)
	@cp src/hal/arch/riscv/linker-$(BOARD).ld src/hal/arch/riscv/linker.ld
	@LOG=${LOG} cargo build --${MODE} --target $(TARGET) --features "board_$(BOARD)"


run:
	qemu-system-riscv64 \
	-machine virt \
	-kernel ../kernel-qemu \
	-m 128M \
	-nographic \
	-smp 2 \
	-bios $(BOOTLOADER) \
#	-drive file=sdcard.img,if=none,format=raw,id=x0  \
#	-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
#	-device virtio-net-device,netdev=net \
#	-netdev user,id=net \
#	-initrd initrd.img


clean:
	@rm src/hal/arch/riscv/linker.ld
	@cargo clean