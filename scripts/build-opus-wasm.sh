#!/usr/bin/env bash

cd lib/opus || exit

if test ! -e "Makefile"; then
    echo "Setting up build files"

    ./autogen.sh
    emconfigure ./configure \
    --disable-rtcd \
    --disable-intrinsics \
    --disable-shared \
    --disable-stack-protector \
    --disable-hardening \
    --disable-doc \
    --disable-extra-programs \
    --enable-static || exit
fi

emmake make -j "$(nproc)"
