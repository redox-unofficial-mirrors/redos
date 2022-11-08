export PARTED?=parted
export QEMU?=qemu-system-x86_64
export REDOS_TARGET=x86_64-unknown-redos

all: $(BUILD)/bootloader.efi

$(BUILD)/lib%.a: redos/%/src/main.rs Cargo.lock $(shell find redos/* -type f)
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
		-m elf_x86_64 \
		--gc-sections \
		-z max-page-size=0x1000 \
		-T linkers/$(REDOS_TARGET).ld \
		-o $@ \
		$<
	cp $@ $@.sym
	$(OBJCOPY) --strip-all $@

$(BUILD)/bootloader.efi: Cargo.lock Cargo.toml $(shell find src -type f)
	mkdir -p $(BUILD)
	env RUSTFLAGS="-C soft-float" \
	cargo rustc \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(TARGET) \
		--bin bootloader \
		--release \
		-- \
		--emit link=$@

$(BUILD)/bootloader-live.efi: Cargo.lock Cargo.toml $(shell find src -type f)
	mkdir -p $(BUILD)
	env RUSTFLAGS="-C soft-float" \
	cargo rustc \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(TARGET) \
		--bin bootloader \
		--release \
		--features live \
		-- \
		--emit link=$@

bloat: $(BUILD)/bootloader.efi
	cp $(BUILD)/bootloader.pdb target/$(TARGET)/release
	env RUSTFLAGS="-C soft-float" \
	cargo bloat \
		-Z build-std=core,alloc,panic_abort \
		-Z build-std-features=compiler-builtins-mem,panic_immediate_abort \
		--target $(TARGET) \
		--bin bootloader \
		--release \
		--crates

$(BUILD)/esp.bin: $(BUILD)/bootloader.efi
	mkdir -p $(BUILD)
	rm -f $@.partial
	fallocate -l 1MiB $@.partial
	mkfs.vfat $@.partial
	mmd -i $@.partial efi
	mmd -i $@.partial efi/boot
	mcopy -i $@.partial $< ::efi/boot/bootx64.efi
	mv $@.partial $@

$(BUILD)/harddrive.bin: $(BUILD)/esp.bin $(BUILD)/filesystem.bin
	mkdir -p $(BUILD)
	rm -f $@.partial
	fallocate -l 11MiB $@.partial
	$(PARTED) -s -a minimal $@.partial mklabel gpt
	$(PARTED) -s -a minimal $@.partial mkpart ESP FAT32 1MiB 2MiB
	$(PARTED) -s -a minimal $@.partial mkpart REDOXFS 2MiB 10MiB
	$(PARTED) -s -a minimal $@.partial set 2 boot on
	dd if=$(BUILD)/esp.bin of=$@.partial bs=1MiB seek=1 conv=notrunc
	dd if=$(BUILD)/filesystem.bin of=$@.partial bs=1MiB seek=2 conv=notrunc
	mv $@.partial $@

$(BUILD)/firmware.rom:
	cp /usr/share/OVMF/OVMF_CODE.fd $@

qemu: $(BUILD)/harddrive.bin $(BUILD)/firmware.rom
	$(QEMU) \
		-d cpu_reset \
		-no-reboot \
		-smp 4 -m 2048 \
		-chardev stdio,id=debug,signal=off,mux=on \
		-serial chardev:debug \
		-mon chardev=debug \
		-machine q35 \
		-net none \
		-enable-kvm \
		-cpu host \
		-bios $(BUILD)/firmware.rom \
		-drive file=$<,format=raw
