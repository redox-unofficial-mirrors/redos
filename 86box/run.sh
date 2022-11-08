#!/usr/bin/env bash

set -e

cd "$(dirname "$0")"

ARCH="$1"
if [ ! -f "${ARCH}.cfg" ]
then
    echo "$0 [i386 or i486]"
    exit 1
fi

export TARGET="${ARCH}-unknown-none"
make -C .. "build/${TARGET}/harddrive.bin"

./86Box-Linux-x86_64-*.AppImage --config "${ARCH}.cfg"
