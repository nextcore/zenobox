#!/bin/bash

# ==============================================================================
# 📦 Zenobox Automated Installer Script
# ==============================================================================
# Script ini mengunduh, mengekstrak, dan memasang Zenobox OCI Container Runtime
# serta otomatis mengonfigurasi symlink pengganti 'docker' & 'docker-compose'.
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
echo "          Zenobox Installer System                "
echo "  Lightweight OCI Container Runtime in Rust       "
echo "=================================================="
echo -e "${NC}"

# 1. System Compatibility Checks
if [ "$(uname -s)" != "Linux" ]; then
    log_error "Zenobox currently only supports Linux operating systems."
    exit 1
fi

ARCH="$(uname -m)"
if [ "$ARCH" != "x86_64" ]; then
    log_error "Architecture '$ARCH' is not supported. Only x86_64 is currently supported."
    exit 1
fi

# 2. Version & Directories Setup
DEFAULT_VERSION="v0.1.0"
DEFAULT_INSTALL_DIR="/opt/zenobox"
SYMLINK_DIR="/usr/local/bin"

VERSION="$DEFAULT_VERSION"
INSTALL_DIR="$DEFAULT_INSTALL_DIR"

while [ $# -gt 0 ]; do
    case "$1" in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--version <version>] [--dir <install_directory>]"
            exit 1
            ;;
    esac
done

if [ -t 0 ]; then
    echo -n "Specify installation directory (default: ${DEFAULT_INSTALL_DIR}): "
    read -r DIR_INPUT
    if [ -n "$DIR_INPUT" ]; then
        INSTALL_DIR="$DIR_INPUT"
    fi

    echo -n "Specify Zenobox target version (default: ${VERSION}): "
    read -r VER_INPUT
    if [ -n "$VER_INPUT" ]; then
        VERSION="$VER_INPUT"
    fi
fi

log_info "Install Directory : ${BOLD}${INSTALL_DIR}${NC}"
log_info "Target Version    : ${BOLD}${VERSION}${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$INSTALL_DIR" "$INSTALL_DIR/bin"
INSTALL_DIR="$(cd "$INSTALL_DIR" && pwd)"
cd "$INSTALL_DIR" || { log_error "Failed to enter directory $INSTALL_DIR"; exit 1; }

# 3. Check for local build binary or download from Github Release
LOCAL_BINARY="${SCRIPT_DIR}/target/release/zenobox"

if [ -f "$LOCAL_BINARY" ]; then
    log_info "Found local release binary at ${LOCAL_BINARY}. Copying..."
    cp "$LOCAL_BINARY" "$INSTALL_DIR/bin/zenobox"
    chmod +x "$INSTALL_DIR/bin/zenobox"
    log_success "Local Zenobox binary installed."
else
    REPO_URL="https://github.com/nextcore/zenobox/releases/download/${VERSION}"
    TARBALL_FILE="zenobox-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"

    log_info "Downloading Zenobox release tarball..."

    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$TARBALL_FILE" -# "${REPO_URL}/${TARBALL_FILE}"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$TARBALL_FILE" "${REPO_URL}/${TARBALL_FILE}"
    else
        log_error "curl or wget is required to download installer package."
        exit 1
    fi

    if [ -f "$TARBALL_FILE" ] && [ -s "$TARBALL_FILE" ]; then
        log_info "Extracting Zenobox package..."
        tar -xzf "$TARBALL_FILE" -C "$INSTALL_DIR/bin/" 2>/dev/null || mv "$TARBALL_FILE" "$INSTALL_DIR/bin/zenobox"
        rm -f "$TARBALL_FILE"
        chmod +x "$INSTALL_DIR/bin/zenobox"
        log_success "Zenobox binary downloaded and extracted."
    else
        log_warn "Could not download remote binary for ${VERSION}."
    fi
fi

# 4. Create Docker & Docker-Compose Symlinks
log_info "Configuring system symlinks in ${SYMLINK_DIR}..."

SUDO_CMD=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
fi

$SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/zenobox" 2>/dev/null || {
    mkdir -p "$HOME/.local/bin"
    SYMLINK_DIR="$HOME/.local/bin"
    ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/zenobox"
}

$SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/docker" 2>/dev/null || {
    ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/docker"
}

$SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/docker-compose" 2>/dev/null || {
    ln -sf "$INSTALL_DIR/bin/zenobox" "$SYMLINK_DIR/docker-compose"
}

log_success "Symlinks configured successfully:"
echo "  - $SYMLINK_DIR/zenobox -> $INSTALL_DIR/bin/zenobox"
echo "  - $SYMLINK_DIR/docker -> $INSTALL_DIR/bin/zenobox"
echo "  - $SYMLINK_DIR/docker-compose -> $INSTALL_DIR/bin/zenobox"

# 5. Configure Shell Aliases (~/.bashrc, ~/.zshrc, ~/.bash_aliases, ~/.profile)
log_info "Configuring shell aliases in shell configuration files..."
ALIAS_DOCKER="alias docker=\"zenobox\""
ALIAS_COMPOSE="alias docker-compose=\"zenobox compose\""

add_alias_if_missing() {
    local rc_file="$1"
    if [ -f "$rc_file" ]; then
        local modified=0
        if ! grep -q "alias docker=" "$rc_file"; then
            echo "" >> "$rc_file"
            echo "# Zenobox Docker Aliases" >> "$rc_file"
            echo "$ALIAS_DOCKER" >> "$rc_file"
            modified=1
        fi
        if ! grep -q "alias docker-compose=" "$rc_file"; then
            if [ $modified -eq 0 ] && ! grep -q "# Zenobox Docker Aliases" "$rc_file"; then
                echo "" >> "$rc_file"
                echo "# Zenobox Docker Aliases" >> "$rc_file"
            fi
            echo "$ALIAS_COMPOSE" >> "$rc_file"
            modified=1
        fi
        if [ $modified -eq 1 ]; then
            log_success "Added docker aliases to $rc_file"
        fi
    fi
}

for RC in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.bash_aliases" "$HOME/.profile"; do
    add_alias_if_missing "$RC"
done

# 6. Verification Check
echo ""
log_info "Verifying Zenobox installation..."
if command -v zenobox >/dev/null 2>&1; then
    VER_OUT="$(zenobox --version 2>&1)"
    log_success "Zenobox is active! (${VER_OUT})"
else
    log_warn "Zenobox installed at $INSTALL_DIR/bin/zenobox. Ensure $SYMLINK_DIR is in your PATH."
fi

# 7. Complete Notice
echo -e "\n=================================================="
log_success "Zenobox Installation Completed!"
echo -e "=================================================="
echo -e "${BOLD}Quick Commands:${NC}"
echo -e "  - Test Docker replacement : ${CYAN}docker --help${NC}"
echo -e "  - Pull container image    : ${CYAN}docker pull alpine${NC}"
echo -e "  - Run container           : ${CYAN}docker run -d -p 8080:80 --name my-nginx nginx:alpine${NC}"
echo -e "  - Run Docker Compose      : ${CYAN}docker-compose up -d${NC}"
echo -e "=================================================="
