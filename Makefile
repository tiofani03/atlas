.PHONY: dev run build clean test

dev:
	@./dev.sh

run: dev

build:
	cargo build
	cd atlas-desktop/frontend && npm run build

clean:
	cargo clean
	rm -rf atlas-desktop/frontend/dist

test:
	cargo test --workspace
