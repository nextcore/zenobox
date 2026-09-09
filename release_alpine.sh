#!/bin/bash

# ==============================================================================
# 🏔️ Zenobox Alpine Linux / MUSL Static Release Publisher
# ==============================================================================
# Script ini memanggil `build_alpine.sh` untuk mengompilasi Zenobox, lalu
# membuat tarball rilis di folder dist/, menghitung SHA256 checksum,
# dan secara opsional menerbitkannya ke GitHub via `gh release`.
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
echo "    Zenobox Alpine (MUSL) Release Publisher       "
echo "=================================================="
echo -e "${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

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

if [ $CLEAN_DIST -eq 1 ]; then
    log_info "Membersihkan direktori rilis (dist/)..."
    rm -rf dist/*
    log_success "Direktori dist/ berhasil dibersihkan."
fi

# 1. Run Build Script
BUILD_ARGS=""
if [ $CLEAN_CACHE -eq 1 ]; then
    BUILD_ARGS="--clean"
fi

log_info "Menjalankan script kompilasi build_alpine.sh..."
bash "${SCRIPT_DIR}/build_alpine.sh" $BUILD_ARGS
if [ $? -ne 0 ]; then
    log_error "Proses kompilasi di build_alpine.sh gagal. Membatalkan rilis."
    exit 1
fi

# 2. Version & Target Detection
GIT_TAG=$(git describe --tags --abbrev=0 2>/dev/null)
if [ -n "$GIT_TAG" ]; then
    VERSION="$GIT_TAG"
else
    CARGO_VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f2 2>/dev/null)
    VERSION="v${CARGO_VERSION:-0.1.0}"
fi
log_info "Detected version: ${BOLD}${VERSION}${NC}"

TARGET="x86_64-unknown-linux-musl"
if [ ! -f "target/${TARGET}/release/zenobox" ] && [ -f "target/release/zenobox" ]; then
    TARGET="x86_64-unknown-linux-gnu"
fi

# 3. Packaging into dist/
DIST_DIR="dist"
PKG_NAME="zenobox-${VERSION}-${TARGET}"
PKG_DIR="${DIST_DIR}/${PKG_NAME}"

mkdir -p "$PKG_DIR"

log_info "Membuat struktur paket rilis di ${PKG_DIR}..."
if [ -f "target/${TARGET}/release/zenobox" ]; then
    cp "target/${TARGET}/release/zenobox" "$PKG_DIR/zenobox"
else
    cp "target/release/zenobox" "$PKG_DIR/zenobox"
fi

cp README.md LICENSE install.sh "$PKG_DIR/" 2>/dev/null

log_info "Membuat tarball rilis..."
cd "$DIST_DIR" || exit 1
TARBALL="${PKG_NAME}.tar.gz"
tar -czf "$TARBALL" "$PKG_NAME"

log_info "Menghasilkan SHA256 checksum..."
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$TARBALL" > "${TARBALL}.sha256"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$TARBALL" > "${TARBALL}.sha256"
fi

rm -rf "$PKG_NAME"
cd - > /dev/null || exit 1

echo -e "\n=================================================="
log_success "Paket Rilis Alpine / MUSL Berhasil Dibuat!"
echo -e "=================================================="
echo -e "  - Tarball Package : ${GREEN}${PWD}/${DIST_DIR}/${TARBALL}${NC}"
echo -e "  - Checksum File   : ${GREEN}${PWD}/${DIST_DIR}/${TARBALL}.sha256${NC}"
echo -e "  - File Size       : ${GREEN}$(du -sh "${DIST_DIR}/${TARBALL}" | cut -f1)${NC}"
echo -e "=================================================="

# 4. Optional GitHub Release Publishing via gh CLI
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    echo ""
    read -p "Publikasikan rilis ini ke GitHub via 'gh release'? (y/N): " PUB_CHOICE
    if [[ "$PUB_CHOICE" =~ ^[yY]$ ]]; then
        log_info "Menerbitkan release ${VERSION} ke GitHub..."
        gh release create "$VERSION" \
            "${DIST_DIR}/${TARBALL}" \
            "${DIST_DIR}/${TARBALL}.sha256" \
            --title "Zenobox ${VERSION} (Alpine/MUSL Static Release)" \
            --notes "Zenobox ${VERSION} static release build for Alpine Linux & glibc systems."
        log_success "GitHub Release berhasil diterbitkan!"
    fi
fi
