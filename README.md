# expi

expi simplifies writing kernels for the Raspberry Pi 3 Model B.

## Build

Install [flatelf] and generate a kernel image:

```
cargo b --release --bin <bin>
flatelf target/aarch64-unknown-none/release/<bin> \
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

Note: check the boot debug output for the exact list.

## Run in QEMU

```
cargo r --release --bin <bin>
```

## Enable Raspberry Pi's first PL011 (UART0)

Raspberry Pi's first PL011 (UART0) can be enabled adding the follow lines to
config.txt.

```
# Disable the Bluetooth device and make the first PL011 (UART0) the primary
# UART.
dtoverlay=disable-bt

# Config the second-stage loader and the main firmware to output diagnostic
# information to UART0.
uart_2ndstage=1
```

The UART will be available on pins [GPIO14 and GPIO15].


[flatelf]: https://github.com/jroimartin/flatelf/
[GPIO14 and GPIO15]: https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#gpio-and-the-40-pin-header
