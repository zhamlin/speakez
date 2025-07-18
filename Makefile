OPUS_DIR  := lib/opus
OPUS_WASM := $(OPUS_DIR)/libopus.wasm

WEB_DIR := ./web/client/js
OPUS_JS_DIR := $(WEB_DIR)/native/opus
OPUS_JS := $(OPUS_JS_DIR)/libopus.js

CRATE_JS := ./crates/web/pkg/speakez_web.js
SPEAKEZ_JS_DIR := $(WEB_DIR)/native/speakez
SPEAKEZ_JS := $(SPEAKEZ_JS_DIR)/speakez_web.js

TS_TYPES := ./crates/web/schemas/speakez.d.ts

KEYS_DIR := ./keys
KEYS_KEY := $(KEYS_DIR)/key.pem
KEYS_CERT := $(KEYS_DIR)/cert.pem

$(KEYS_KEY):
	mkdir -p $(KEYS_DIR)
	openssl req -nodes -newkey rsa:4096 -x509 -keyout $(KEYS_KEY) -sha256 -out $(KEYS_CERT)

PHONY: run-server
run-server: $(KEYS_KEY)
	cargo run --bin speakez-server

PHONY: run-web
run-web: PORT=8080
run-web: $(SPEAKEZ_JS) $(OPUS_JS)
	cd ./web/proxy; go run ./... -dir=../client -pattern=".*(.html|.js|.css)" -addr localhost:$(PORT)

PHONY: ts-types
ts-types: $(TS_TYPES)

$(TS_TYPES):
	mkdir -p ./crates/client/schemas
	cargo test -p speakez-client --features jsonschema
	biome format --stdin-file-path test.ts > $(TS_TYPES) < ./crates/client/schemas/types.d.ts

$(CRATE_JS): $(TS_TYPES)
	cd ./crates/web; ./scripts/build.sh

$(SPEAKEZ_JS): $(CRATE_JS)
	mkdir -p $(SPEAKEZ_JS_DIR)
	cp -rf ./crates/web/pkg/* $(SPEAKEZ_JS_DIR)

$(OPUS_DIR)/autogen.sh:
	git submodule update --init --recursive

$(OPUS_WASM): $(OPUS_DIR)/autogen.sh
	./scripts/build-opus-wasm.sh
	./scripts/link-opus-wasm.sh

$(OPUS_JS): $(OPUS_WASM)
	mkdir -p $(OPUS_JS_DIR)
	cp $(OPUS_DIR)/libopus.{js,wasm} $(OPUS_JS_DIR)
