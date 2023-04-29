export LD?=ld
export OBJCOPY?=objcopy
export PARTED?=parted
export QEMU?=qemu-system-i386
export REDOS_TARGET=i386-unknown-redos

all: $(BUILD)/harddrive.bin

.SECONDEXPANSION:

$(BUILD)/lib%.a: $$(shell find redos/$$* redos/abi redos/hw Cargo.lock -type f)
	env | sort
	mkdir -p $(BUILD)
	env RUSTFLAGS="-C lto=fat -C soft-float" \
	cargo rustc \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(REDOS_TARGET) \
		--lib \
		--release \
		--package redos_$* \
		-- \
		--emit link=$@

$(BUILD)/%.elf: $(BUILD)/lib%.a linkers/$(REDOS_TARGET).ld
	mkdir -p $(BUILD)
	$(LD) \
		-m elf_i386 \
		--gc-sections \
		-z max-page-size=0x1000 \
		-T linkers/$(REDOS_TARGET).ld \
		-o $@ \
		$<
	cp $@ $@.sym
	$(OBJCOPY) --strip-all $@

$(BUILD)/libbootloader.a: Cargo.lock Cargo.toml $(shell find src -type f)
	mkdir -p $(BUILD)
	env RUSTFLAGS="-C lto=fat -C soft-float" \
	cargo rustc \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(TARGET) \
		--lib \
		--release \
		-- \
		--emit link=$@

$(BUILD)/bootloader.elf: linkers/$(TARGET).ld $(BUILD)/libbootloader.a
	mkdir -p $(BUILD)
	$(LD) -m elf_i386 --gc-sections -z max-page-size=0x1000 -T $< -o $@ $(BUILD)/libbootloader.a
	cp $@ $@.sym
	$(OBJCOPY) --strip-all $@

$(BUILD)/bootloader.bin: $(BUILD)/bootloader.elf $(shell find asm/x86 -type f)
	mkdir -p $(BUILD)
	nasm -f bin -o $@ -l $@.lst -D STAGE3=$< -iasm/x86/ asm/x86/bootloader.asm

$(BUILD)/libbootloader-live.a: Cargo.lock Cargo.toml $(shell find src -type f)
	mkdir -p $(BUILD)
	env RUSTFLAGS="-C lto=fat -C soft-float" \
	cargo rustc \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(TARGET) \
		--lib \
		--release \
		--features live \
		-- \
		--emit link=$@

$(BUILD)/bootloader-live.elf: linkers/$(TARGET).ld $(BUILD)/libbootloader-live.a
	mkdir -p $(BUILD)
	$(LD) -m elf_i386 --gc-sections -z max-page-size=0x1000 -T $< -o $@ $(BUILD)/libbootloader-live.a && \
	cp $@ $@.sym && \
	$(OBJCOPY) --strip-all $@

$(BUILD)/bootloader-live.bin: $(BUILD)/bootloader-live.elf $(shell find asm/x86 -type f)
	mkdir -p $(BUILD)
	nasm -f bin -o $@ -l $@.lst -D STAGE3=$< -iasm/x86/ asm/x86/bootloader.asm

$(BUILD)/harddrive.bin: $(BUILD)/bootloader.bin $(BUILD)/filesystem.bin
	mkdir -p $(BUILD)
	rm -f $@.partial
	fallocate -l 11MiB $@.partial
	$(PARTED) -s -a minimal $@.partial mklabel msdos
	$(PARTED) -s -a minimal $@.partial mkpart primary 2MiB 10MiB
	dd if=$< of=$@.partial bs=1 count=446 conv=notrunc
	dd if=$< of=$@.partial bs=512 skip=1 seek=1 conv=notrunc
	dd if=$(BUILD)/filesystem.bin of=$@.partial bs=1MiB seek=2 conv=notrunc
	mv $@.partial $@

qemu: $(BUILD)/harddrive.bin
	$(QEMU) \
		-d cpu_reset,guest_errors,int \
		-no-reboot \
		-smp 1 -m 8 \
		-audiodev sdl,id=audio0 \
		-chardev stdio,id=debug,signal=off,mux=on \
		-serial chardev:debug \
		-mon chardev=debug \
		-machine pc,pcspk-audiodev=audio0 \
		-net none \
		-cpu 486 \
		-drive file=$<,format=raw
