# ReDOS

ReDOS is a disk operating system that utilizes technology from Redos OS but
focuses on support for systems with very low specifications. It can run on
systems with as low as 2MiB of memory and a 386 processor. It uses RedoxFS
as the filesystem, which supports copy-on-write atomicity. Programs can be
written in Rust or Rhai, and there is a simple ABI to utilize features provided
by the kernel.

ReDOS can be built with `make`. The default target is `i486-unknown-none`, which
supports BIOS systems with an Intel 486 or better processor. You can also build
for Intel 386 processors, which disables support for the Rhai interpreter, with
`make TARGET=i386-unknown-none`. There is a harddrive image in
`build/TARGET/harddrive.bin` that can be copied to storage media of your choice
for booting real systems. You may also run ReDOS in an emulator with
`make qemu`.
