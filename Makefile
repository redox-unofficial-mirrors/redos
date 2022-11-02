TARGET?=x86_64-unknown-uefi
BUILD=build/$(TARGET)
export RUST_TARGET_PATH=$(CURDIR)/targets

include mk/$(TARGET).mk

clean:
	rm -rf build target

$(BUILD)/filesystem: redos/autoexec.rhai
	mkdir -p $(BUILD)
	rm -f $@.partial
	mkdir $@.partial
	mkdir $@.partial/redos
	cp $< $@.partial/redos/autoexec.rhai
	mv $@.partial $@

$(BUILD)/filesystem.bin: $(BUILD)/filesystem
	mkdir -p $(BUILD)
	rm -f $@.partial
	redoxfs-ar $@.partial $<
	mv $@.partial $@
