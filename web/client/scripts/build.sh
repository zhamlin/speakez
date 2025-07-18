#!/usr/bin/env bash

esbuild css/styles.css --minify >| dist/styles.css

esbuild js/index.js \
    --define:DEBUG=false \
    --minify --bundle --format=esm \
    --alias:preact=./js/third_party/preact/preact.js \
    --alias:preact/hooks=./js/third_party/preact/preact-hooks.js \
    --alias:@preact/signals=./js/third_party/preact/signals.js \
    --alias:@preact/signals-core=./js/third_party/preact/signals-core.js \
    --alias:htm=./js/third_party/htm.js \
>| dist/index.js

cp ./js/native/opus/libopus.wasm dist/
