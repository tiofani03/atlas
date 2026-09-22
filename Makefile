.PHONY: dev run build clean test install-cli desktop desktop-build desktop-stop desktop-restart desktop-status

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

install-cli:
	cargo install --path atx --force

# Desktop Web UI + Axum backend (embedded frontend, port 31415)
desktop-build:
	cargo build --bin atx

desktop:
	./target/debug/atx ui --no-open

desktop-stop:
	@pids="$$(lsof -tiTCP:31415 -sTCP:LISTEN 2>/dev/null || true)"; \
	if [ -n "$$pids" ]; then kill $$pids; fi

desktop-restart: desktop-stop desktop-build
	@setsid nohup ./target/debug/atx ui --no-open >/tmp/atlas-ui.log 2>&1 & echo "Atlas UI started (logs: /tmp/atlas-ui.log)"

desktop-status:
	@curl -fsS http://localhost:31415/api/status
	@echo
