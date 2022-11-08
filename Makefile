TARGET?=i386-unknown-none
BUILD=build/$(TARGET)
export RUST_TARGET_PATH=$(CURDIR)/targets

REDOS_BINS=\
	$(BUILD)/date.elf \
	$(BUILD)/help.elf \
	$(BUILD)/vi.elf
#TODO $(BUILD)/rhai.elf (needs atomic cas)

include mk/$(TARGET).mk

clean:
	rm -rf build target

$(BUILD)/filesystem: redos/autoexec redos/*.rhai redos/*.txt $(REDOS_BINS)
	mkdir -p $(BUILD)
	rm -f $@.partial
	mkdir $@.partial
	mkdir $@.partial/redos
	for file in $^; do cp $$file $@.partial/redos; done
	mv $@.partial $@

$(BUILD)/filesystem.bin: $(BUILD)/filesystem
	mkdir -p $(BUILD)
	rm -f $@.partial
	fallocate -l 8MiB $@.partial
	redoxfs-ar $@.partial $<
	mv $@.partial $@
