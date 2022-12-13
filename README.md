# expi

expi simplifies writing kernels for the Raspberry Pi 3 Model B.

## Build

Install [flatelf] and generate a kernel image:

```
cd <kernel>
cargo build --release
flatelf target/aarch64-unknown-none/release/<kernel> \
    target/aarch64-unknown-none/release/kernel8.img
```

Then, boot it via PXE or copy it into a SD card. You also need the following
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
cd <kernel>
cargo run --release
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

## Network boot setup

Run dnsmasq with the following configuration:

```
port=0
interface=eth0
dhcp-range=10.0.0.100,10.0.0.200
log-dhcp
enable-tftp
tftp-root=/var/tftproot
pxe-service=0,"Raspberry Pi Boot"
```

You might need to adjust the `interface` and `dhcp-range` parameters depending
on your network setup. You also have to copy the files mentioned in the "Build"
section into `/var/tftproot`.


[flatelf]: https://github.com/jroimartin/flatelf/
[GPIO14 and GPIO15]: https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#gpio-and-the-40-pin-header
