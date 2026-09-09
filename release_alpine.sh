#!/bin/bash

# ==============================================================================
# 🏔️ Zenobox Alpine Linux / MUSL Static Release Compiler & Publisher
# ==============================================================================
# Script ini mengompilasi Zenobox secara statically-linked untuk Alpine Linux
# (x86_64-unknown-linux-musl), memotong simbol (strip), membuat tarball rilis,
# dan menghasilkan checksum SHA256.
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
echo "    Zenobox Alpine (MUSL) Release Compiler        "
echo "=================================================="
echo -e "${NC}"

# Parse Command Line Arguments
CLEAN_CACHE=0
CLEAN_DIST=0

while [ $# -gt 0 ]; do
    case "$1" in
        --clean|-c)
            CLEAN_CACHE=1
            shift
            ;;
        --clean-dist)
            CLEAN_DIST=1
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --clean, -c      Hapus cache build Cargo (cargo clean) sebelum mengompilasi"
            echo "  --clean-dist     Hapus artefak rilis lama di folder dist/ sebelum mengompilasi"
            echo "  --help, -h       Tampilkan bantuan ini"
            exit 0
            ;;
        *)
            log_error "Opsi tidak dikenal: $1"
            echo "Penggunaan: $0 [--clean|-c] [--clean-dist] [--help|-h]"
            exit 1
            ;;
    esac
done

# Perform Cleanup if requested
if [ $CLEAN_CACHE -eq 1 ]; then
    log_info "Membersihkan cache kompilasi Cargo (cargo clean)..."
    cargo clean
    log_success "Cache kompilasi berhasil dibersihkan."
fi

if [ $CLEAN_DIST -eq 1 ]; then
    log_info "Membersihkan direktori rilis (dist/)..."
    rm -rf dist/*
    log_success "Direktori dist/ berhasil dibersihkan."
fi

# 1. Version Detection
GIT_TAG=$(git describe --tags --abbrev=0 2>/dev/null)
if [ -n "$GIT_TAG" ]; then
    VERSION="$GIT_TAG"
else
    CARGO_VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f2 2>/dev/null)
    VERSION="v${CARGO_VERSION:-0.1.0}"
fi
log_info "Detected version: ${BOLD}${VERSION}${NC}"

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
log_info "Compiling Zenobox static binary for $TARGET..."

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
log_success "Compilation succeeded for $TARGET."

# 4. Packaging into dist/
DIST_DIR="dist"
PKG_NAME="zenobox-${VERSION}-${TARGET}"
PKG_DIR="${DIST_DIR}/${PKG_NAME}"

mkdir -p "$PKG_DIR"

log_info "Copying binaries and release assets..."
if [ -f "target/${TARGET}/release/zenobox" ]; then
    cp "target/${TARGET}/release/zenobox" "$PKG_DIR/zenobox"
else
    cp "target/release/zenobox" "$PKG_DIR/zenobox"
fi

cp README.md LICENSE install.sh "$PKG_DIR/" 2>/dev/null

log_info "Stripping binary symbols to optimize size..."
strip "$PKG_DIR/zenobox" 2>/dev/null || log_warn "strip tool not available, skipping."

log_info "Creating distribution tarball..."
cd "$DIST_DIR" || exit 1
TARBALL="${PKG_NAME}.tar.gz"
tar -czf "$TARBALL" "$PKG_NAME"

log_info "Generating SHA256 checksum..."
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$TARBALL" > "${TARBALL}.sha256"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$TARBALL" > "${TARBALL}.sha256"
fi

rm -rf "$PKG_NAME"
cd - > /dev/null || exit 1

echo -e "\n=================================================="
log_success "Alpine / MUSL Static Release Package Created!"
echo -e "=================================================="
echo -e "  - Tarball Package : ${GREEN}${PWD}/${DIST_DIR}/${TARBALL}${NC}"
echo -e "  - Checksum File   : ${GREEN}${PWD}/${DIST_DIR}/${TARBALL}.sha256${NC}"
echo -e "  - File Size       : ${GREEN}$(du -sh "${DIST_DIR}/${TARBALL}" | cut -f1)${NC}"
echo -e "=================================================="

# 5. Optional GitHub Release Publishing via gh CLI
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    echo ""
    read -p "Publish this release to GitHub via 'gh release'? (y/N): " PUB_CHOICE
    if [[ "$PUB_CHOICE" =~ ^[yY]$ ]]; then
        log_info "Publishing release ${VERSION} to GitHub..."
        gh release create "$VERSION" \
            "${DIST_DIR}/${TARBALL}" \
            "${DIST_DIR}/${TARBALL}.sha256" \
            --title "Zenobox ${VERSION} (Alpine/MUSL Static Release)" \
            --notes "Zenobox ${VERSION} static release build for Alpine Linux & glibc systems."
        log_success "GitHub Release published successfully!"
    fi
fi
