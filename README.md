# ExPI

Experimental OS for Raspberry Pi 3 Model B.

## Build

Install [flatelf] and generate the kernel image:

```
cargo b --release
flatelf target/aarch64-unknown-none/release/expi \
    target/aarch64-unknown-none/release/kernel8.img
```

Then, serve it via iPXE or copy it into a SD card. You also need the following
files from the raspios' boot partition:

- start.elf
- bootcode.bin
- config.txt
- fixup.dat
- bcm2710-rpi-3-b.dtb
- overlays/vc4-kms-v3d.dtbo
- overlays/disable-bt.dtbo

Note: you should check the boot debug output for the exact list.

## Run in QEMU

```
cargo r --release
```


[flatelf]: https://github.com/jroimartin/flatelf/
