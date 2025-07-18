#!/usr/bin/env bash

cd lib/opus || exit

exports=$(cat <<EOF
[
  '_free',
  '_malloc',
  '_opus_strerror',
  '_opus_get_version_string',
  '_opus_encoder_get_size',
  '_opus_encoder_init',
  '_opus_encode',
  '_opus_encode_float',
  '_opus_encoder_ctl',
  '_opus_decoder_get_size',
  '_opus_decoder_init',
  '_opus_decode',
  '_opus_decode_float',
  '_opus_decoder_ctl',
  '_opus_decoder_create',
  '_opus_packet_get_nb_samples',
  '_opus_packet_get_nb_channels'
]
EOF
)

# EMCC_OPTS=-O3 --memory-init-file 0
emcc -o libopus.js ./.libs/libopus.a \
  -O2 \
  --closure 1 \
  -flto=full \
  -s EXPORT_ES6=1 \
  -s MODULARIZE=1 \
  -s NO_FILESYSTEM=1 \
  -s EXPORT_NAME='createOpusModule' \
  -s EXPORTED_FUNCTIONS="${exports}" \
  -s EXPORTED_RUNTIME_METHODS='["cwrap", "getValue", "setValue", "UTF8ToString"]'
