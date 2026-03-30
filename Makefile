.PHONY: wasm

wasm:
	wasm-pack.exe build --release --target web --no-default-features
	cargo install simple-http-server
	simple-http-server -ip 80