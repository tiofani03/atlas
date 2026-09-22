#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "=========================================="
echo "  🌟 Starting Atlas Desktop (Full Stack)  "
echo "=========================================="

# Free ports if previously occupied
if lsof -ti :31415 >/dev/null 2>&1; then
  echo "Freeing port 31415..."
  lsof -ti :31415 | xargs -r kill -9 2>/dev/null || true
fi

if lsof -ti :31420 >/dev/null 2>&1; then
  echo "Freeing port 31420..."
  lsof -ti :31420 | xargs -r kill -9 2>/dev/null || true
fi

# 1. Start backend
echo "▶ Starting Backend Server (Rust / Axum on port 31415)..."
cargo run --bin atlas-desktop-backend &
BACKEND_PID=$!

# 2. Start frontend
echo "▶ Starting Frontend Server (React 19 / Vite on port 31420)..."
(cd "$DIR/atlas-desktop/frontend" && npm run dev -- --host 0.0.0.0 --port 31420) &
FRONTEND_PID=$!

cleanup() {
  echo ""
  echo "🛑 Stopping Atlas Desktop..."
  kill -TERM "$BACKEND_PID" 2>/dev/null || true
  kill -TERM "$FRONTEND_PID" 2>/dev/null || true
  # Also kill child node/cargo processes if any
  lsof -ti :31415 | xargs -r kill -9 2>/dev/null || true
  lsof -ti :31420 | xargs -r kill -9 2>/dev/null || true
  exit 0
}

trap cleanup SIGINT SIGTERM EXIT

echo ""
echo "✨ Atlas Desktop is launching!"
echo "   - Frontend Web UI:  http://localhost:31420"
echo "   - Backend API:       http://127.0.0.1:31415"
echo "   Press Ctrl+C to stop both servers."
echo ""

wait
