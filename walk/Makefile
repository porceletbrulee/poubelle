
.PHONY: install
install: wasm	
	cp target/wasm32-unknown-unknown/debug/walk.wasm web/

.PHONY: wasm
wasm:
	cargo build --target wasm32-unknown-unknown

.PHONY: test
test:
	RUST_BACKTRACE=1 cargo test

.PHONY: run
run:
	cd web && python3 run.py
