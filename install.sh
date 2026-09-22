#!/usr/bin/env bash
# Atlas CLI (`atx`) Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/tiofani03/atlas/main/install.sh | bash

set -euo pipefail

REPO="tiofani03/atlas"
BIN_NAME="atx"

# Colors
C_RESET='\033[0m'
C_BOLD='\033[1m'
C_GREEN='\033[32m'
C_BLUE='\033[34m'
C_YELLOW='\033[33m'
C_RED='\033[31m'

info() {
    printf "${C_BLUE}${C_BOLD}[INFO]${C_RESET} %s\n" "$1"
}

success() {
    printf "${C_GREEN}${C_BOLD}[OK]${C_RESET} %s\n" "$1"
}

warn() {
    printf "${C_YELLOW}${C_BOLD}[WARN]${C_RESET} %s\n" "$1"
}

error() {
    printf "${C_RED}${C_BOLD}[ERROR]${C_RESET} %s\n" "$1" >&2
    exit 1
}

# 1. Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        OS_TARGET="unknown-linux-gnu"
        ;;
    Darwin)
        OS_TARGET="apple-darwin"
        ;;
    *)
        error "Unsupported operating system: $OS. Atlas currently supports Linux and macOS."
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_TARGET="x86_64"
        ;;
    aarch64|arm64)
        ARCH_TARGET="aarch64"
        ;;
    *)
        error "Unsupported CPU architecture: $ARCH. Atlas currently supports x86_64 and aarch64."
        ;;
esac

TARGET="${ARCH_TARGET}-${OS_TARGET}"
info "Detected platform: ${TARGET}"

# 2. Determine target installation directory
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

# 3. Determine version to install
if [ -z "${ATX_VERSION:-}" ]; then
    info "Resolving latest Atlas release from GitHub..."
    LATEST_JSON=$(curl -fsSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)
    
    if [ -n "$LATEST_JSON" ]; then
        VERSION=$(echo "$LATEST_JSON" | grep -o '"tag_name": *"[^"]*"' | head -1 | cut -d'"' -f4)
    else
        VERSION=""
    fi

    if [ -z "$VERSION" ]; then
        # Fallback to redirect header
        VERSION=$(curl -fsSL -I "https://github.com/${REPO}/releases/latest" 2>/dev/null | grep -i '^location:' | tr -d '\r' | awk -F'/' '{print $NF}' || true)
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version from GitHub releases. You can specify a version via ATX_VERSION=vX.Y.Z."
    fi
else
    VERSION="$ATX_VERSION"
fi

info "Selected version: ${VERSION}"

# 4. Download and Extract Binary Archive
ARCHIVE_NAME="atx-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARCHIVE_NAME}"

TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT

info "Downloading ${DOWNLOAD_URL}..."
if ! curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${ARCHIVE_NAME}"; then
    error "Failed to download binary archive from ${DOWNLOAD_URL}."
fi

info "Extracting ${ARCHIVE_NAME}..."
tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "$TMP_DIR"

EXTRACTED_BIN=$(find "$TMP_DIR" -type f -name "$BIN_NAME" | head -n 1)
if [ -z "$EXTRACTED_BIN" ]; then
    error "Binary '${BIN_NAME}' not found inside downloaded archive."
fi

mv "$EXTRACTED_BIN" "${INSTALL_DIR}/${BIN_NAME}"
chmod +x "${INSTALL_DIR}/${BIN_NAME}"

success "Atlas CLI installed successfully to ${INSTALL_DIR}/${BIN_NAME}"

# 5. Check PATH and verify installation
if ! echo ":$PATH:" | grep -q ":${INSTALL_DIR}:"; then
    warn "${INSTALL_DIR} is not in your current PATH."
    printf "\nTo add it to your PATH, append this to your shell profile (~/.bashrc, ~/.zshrc, etc.):\n"
    printf "  ${C_BOLD}export PATH=\"%s:\$PATH\"${C_RESET}\n\n" "$INSTALL_DIR"
fi

# Run version check
if command -v "${INSTALL_DIR}/${BIN_NAME}" >/dev/null 2>&1; then
    printf "\n"
    "${INSTALL_DIR}/${BIN_NAME}" --version || true
    printf "\n"
fi

success "Done! Initialize Atlas with: ${C_BOLD}atx init${C_RESET}"
