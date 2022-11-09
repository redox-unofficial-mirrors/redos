# ReDOS

ReDOS is a disk operating system that utilizes technology from Redos OS but
focuses on support for systems with very low specifications. It can run on
systems with as low as 2MiB of memory and a 386 processor. It uses
[RedoxFS](https://gitlab.redos-os.org/redos-os/redoxfs) as the filesystem, which
supports copy-on-write atomicity. Programs can be written in
[Rust](https://www.rust-lang.org/) or [Rhai](https://rhai.rs), and there is a
simple ABI to utilize features provided by the kernel.

## Building

ReDOS can be built with `make`. The default target is `i486-unknown-none`, which
supports BIOS systems with an Intel 486 or better processor:

```
make TARGET=i486-unknown-none
```

You can also build for Intel 386 processors, which disables support for the Rhai
interpreter, with:

```
make TARGET=i386-unknown-none
```

## Running

There is a harddrive image in `build/TARGET/harddrive.bin` that can be copied to
storage media of your choice for booting real systems. You may also run ReDOS in
an emulator with `make qemu`.

Additionally, there is support for cycle-accurate emulation with
[86Box](https://86box.net/).
  - Download the latest 86Box AppImage and place it in the `86box` folder
  - Download the latest 86Box roms zip file and extract it to the `86box` folder
  - Run `86bos/run.sh i386` or `86box/run.sh i486` to emulate a system with that CPU
