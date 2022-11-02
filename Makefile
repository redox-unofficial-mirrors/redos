TARGET?=x86_64-unknown-uefi
BUILD=build/$(TARGET)
export RUST_TARGET_PATH=$(CURDIR)/targets

include mk/$(TARGET).mk

clean:
	rm -rf build target

$(BUILD)/filesystem: autoexec.rhai
	mkdir -p $(BUILD)
	rm -f $@.partial
	mkdir $@.partial
	mkdir $@.partial/boot
	cp autoexec.rhai $@.partial/autoexec.rhai
	mv $@.partial $@

$(BUILD)/filesystem.bin: $(BUILD)/filesystem
	mkdir -p $(BUILD)
	rm -f $@.partial
	redoxfs-ar $@.partial $<
	mv $@.partial $@
