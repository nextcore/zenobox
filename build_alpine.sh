#!/bin/bash

# ==============================================================================
# 🏔️ Zenobox Alpine Linux / MUSL Static Builder
# ==============================================================================
# Script ini mengompilasi Zenobox secara statically-linked untuk Alpine Linux
# (x86_64-unknown-linux-musl) dan memotong simbol (strip).
# ==============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[i]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

echo -e "${CYAN}${BOLD}"
echo "=================================================="
echo "      Zenobox Alpine (MUSL) Static Builder        "
echo "=================================================="
echo -e "${NC}"

CLEAN_CACHE=0
MANUAL_VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --clean|-c)
            CLEAN_CACHE=1
            shift
            ;;
        --version|-v)
            MANUAL_VERSION="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --version, -v <ver> Set versi rilis secara manual (misal: v0.3.0)"
            echo "  --clean, -c         Hapus cache build Cargo (cargo clean) sebelum mengompilasi"
            echo "  --help, -h          Tampilkan bantuan ini"
            exit 0
            ;;
        *)
            log_error "Opsi tidak dikenal: $1"
            echo "Penggunaan: $0 [--version|-v <ver>] [--clean|-c] [--help|-h]"
            exit 1
            ;;
    esac
done

if [ $CLEAN_CACHE -eq 1 ]; then
    log_info "Membersihkan cache kompilasi Cargo (cargo clean)..."
    cargo clean
    log_success "Cache kompilasi berhasil dibersihkan."
fi

# 1. Version Detection
if [ -n "$MANUAL_VERSION" ]; then
    VERSION="$MANUAL_VERSION"
    log_info "Using manual version: ${BOLD}${VERSION}${NC}"
else
    GIT_TAG=$(git describe --tags --abbrev=0 2>/dev/null)
    if [ -n "$GIT_TAG" ]; then
        VERSION="$GIT_TAG"
    else
        CARGO_VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f2 2>/dev/null)
        VERSION="v${CARGO_VERSION:-0.1.0}"
    fi
    log_info "Detected version: ${BOLD}${VERSION}${NC}"
fi

TARGET="x86_64-unknown-linux-musl"
log_info "Target architecture: ${BOLD}${TARGET}${NC} (Alpine Linux Fully Static Binary)"

# 2. Rust & Toolchain Checks
if ! command -v cargo >/dev/null 2>&1; then
    log_error "Rust/Cargo is not installed. Please install Rust first."
    exit 1
fi

if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    log_info "Installing Rust target '$TARGET' via rustup..."
    rustup target add "$TARGET" || log_warn "Failed to add target via rustup."
fi

# Detect C compiler for MUSL target
MUSL_CC=""
if command -v musl-gcc >/dev/null 2>&1; then
    MUSL_CC="musl-gcc"
elif command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
    MUSL_CC="x86_64-linux-musl-gcc"
elif command -v gcc >/dev/null 2>&1; then
    MUSL_CC="gcc"
fi

if [ -n "$MUSL_CC" ]; then
    log_info "Using C compiler for MUSL: ${BOLD}${MUSL_CC}${NC}"
    export CC_x86_64_unknown_linux_musl="$MUSL_CC"
else
    log_warn "No C compiler (musl-gcc/gcc) found. Installation of musl-tools or gcc is recommended."
fi

# 3. Compilation
log_info "Compiling Zenobox static binary for $TARGET ($VERSION)..."

cargo build --release --target "$TARGET"
BUILD_STATUS=$?

if [ $BUILD_STATUS -ne 0 ]; then
    log_warn "MUSL target compilation failed. Falling back to standard release build..."
    cargo build --release
    BUILD_STATUS=$?
    TARGET="x86_64-unknown-linux-gnu"
fi

if [ $BUILD_STATUS -ne 0 ]; then
    log_error "Compilation failed!"
    echo "Tip: On Debian/Ubuntu host, install musl-tools: 'sudo apt install musl-tools'"
    exit 1
fi

# 4. Strip Symbols
BIN_PATH="target/${TARGET}/release/zenobox"
if [ ! -f "$BIN_PATH" ]; then
    BIN_PATH="target/release/zenobox"
fi

log_info "Stripping binary symbols to optimize size..."
strip "$BIN_PATH" 2>/dev/null || log_warn "strip tool not available, skipping."

echo -e "\n=================================================="
log_success "Zenobox Static Binary Built Successfully! (${VERSION})"
echo -e "=================================================="
echo -e "  - Version         : ${GREEN}${VERSION}${NC}"
echo -e "  - Binary Location : ${GREEN}${PWD}/${BIN_PATH}${NC}"
echo -e "  - Binary Size     : ${GREEN}$(du -sh "${BIN_PATH}" | cut -f1)${NC}"
echo -e "=================================================="
