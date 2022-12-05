#!/bin/bash

if [[ $# -ne 1 ]]; then
	echo "usage: $0 bin" >&2
	exit 2
fi

exec aarch64-linux-gnu-objdump \
	-b binary \
	-m aarch64 \
	--adjust-vma 0x80000 \
	-D \
	"${1}"
