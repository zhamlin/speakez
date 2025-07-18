#!/usr/bin/env bash

if [ "$1" = "release" ]; then
    wasm-pack build \
        --target web \
        --reference-types \
        --weak-refs \
        --release
else
    wasm-pack build \
    --target web \
    --reference-types \
    --weak-refs \
    --dev \
    -- --features panic_hook
fi
