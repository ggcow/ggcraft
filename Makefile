.PHONY: all wasm install

all: install wasm

wasm:
	wasm-pack build --release --target web --no-default-features
	simple-http-server -ip 80

install:
	cargo install wasm-pack
	cargo install simple-http-server