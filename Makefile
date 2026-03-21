export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: dev build test test-mcp lint fmt clean bench bundle

dev:
	cd frontend && npm run dev &
	cargo tauri dev

build:
	cd frontend && npm run build
	cargo tauri build

test:
	cargo test --workspace

test-mcp:
	cargo test -p lattice-mcp

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean
	cd frontend && rm -rf node_modules dist

bench:
	cargo bench --workspace

bundle:
	cd frontend && npm run build
	cargo tauri build --release
