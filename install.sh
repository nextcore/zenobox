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
DEFAULT_VERSION="v0.2.6"
DEFAULT_INSTALL_DIR="/opt/zenobox"
SYMLINK_DIR="/usr/local/bin"

VERSION="$DEFAULT_VERSION"
INSTALL_DIR="$DEFAULT_INSTALL_DIR"
USE_LOCAL=0

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
        --local)
            USE_LOCAL=1
            shift
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--version <version>] [--dir <install_directory>] [--local]"
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

# Detect Sudo/Root requirements
SUDO_CMD=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
fi

log_info "Install Directory : ${BOLD}${INSTALL_DIR}${NC}"
log_info "Target Version    : ${BOLD}${VERSION}${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Ensure target directories exist with proper permissions
$SUDO_CMD mkdir -p "$INSTALL_DIR" "$INSTALL_DIR/bin" || {
    log_error "Failed to create installation directory '$INSTALL_DIR'. Please run the script with sudo: 'sudo ./install.sh'"
    exit 1
}

# Resolve absolute path safely
if [ -d "$INSTALL_DIR" ]; then
    INSTALL_DIR="$(cd "$INSTALL_DIR" && pwd)"
else
    log_error "Installation directory '$INSTALL_DIR' could not be accessed."
    exit 1
fi

cd "$INSTALL_DIR" || { log_error "Failed to enter directory $INSTALL_DIR"; exit 1; }

# 3. Download from GitHub Release (or optional local build)
LOCAL_MUSL="${SCRIPT_DIR}/target/x86_64-unknown-linux-musl/release/zenobox"
LOCAL_GNU="${SCRIPT_DIR}/target/release/zenobox"

if [ $USE_LOCAL -eq 1 ] && [ -f "$LOCAL_MUSL" ]; then
    log_info "Found local MUSL release binary at ${LOCAL_MUSL}. Copying..."
    $SUDO_CMD cp "$LOCAL_MUSL" "$INSTALL_DIR/bin/zenobox"
    $SUDO_CMD chmod +x "$INSTALL_DIR/bin/zenobox"
    log_success "Local Zenobox static binary installed."
elif [ $USE_LOCAL -eq 1 ] && [ -f "$LOCAL_GNU" ]; then
    log_info "Found local release binary at ${LOCAL_GNU}. Copying..."
    $SUDO_CMD cp "$LOCAL_GNU" "$INSTALL_DIR/bin/zenobox"
    $SUDO_CMD chmod +x "$INSTALL_DIR/bin/zenobox"
    log_success "Local Zenobox binary installed."
else
    REPO_URL="https://github.com/nextcore/zenobox/releases/download/${VERSION}"
    TARBALL_MUSL="zenobox-${VERSION}-x86_64-unknown-linux-musl.tar.gz"
    TARBALL_GNU="zenobox-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"

    log_info "Downloading Zenobox release tarball from GitHub (${REPO_URL})..."

    DOWNLOAD_SUCCESS=0
    TARBALL_FILE=""

    download_file() {
        local file="$1"
        if command -v curl >/dev/null 2>&1; then
            curl -f -L -o "$file" -# "${REPO_URL}/${file}"
        elif command -v wget >/dev/null 2>&1; then
            wget -q -O "$file" "${REPO_URL}/${file}"
        fi
    }

    log_info "Attempting to download static MUSL release package (${TARBALL_MUSL})..."
    if download_file "$TARBALL_MUSL" && [ -s "$TARBALL_MUSL" ]; then
        TARBALL_FILE="$TARBALL_MUSL"
        DOWNLOAD_SUCCESS=1
    else
        log_info "MUSL release package not found, trying GNU release package (${TARBALL_GNU})..."
        if download_file "$TARBALL_GNU" && [ -s "$TARBALL_GNU" ]; then
            TARBALL_FILE="$TARBALL_GNU"
            DOWNLOAD_SUCCESS=1
        fi
    fi

    if [ $DOWNLOAD_SUCCESS -eq 1 ] && [ -n "$TARBALL_FILE" ]; then
        log_info "Extracting Zenobox package..."
        # Extract binary directly into bin/
        $SUDO_CMD tar -xzf "$TARBALL_FILE" --strip-components=1 -C "$INSTALL_DIR/bin/" 2>/dev/null || \
        $SUDO_CMD tar -xzf "$TARBALL_FILE" -C "$INSTALL_DIR/bin/" 2>/dev/null || \
        $SUDO_CMD mv "$TARBALL_FILE" "$INSTALL_DIR/bin/zenobox"

        # Fix nested binary path if extracted with directory prefix
        for nested in "$INSTALL_DIR/bin/zenobox-"*/zenobox; do
            if [ -f "$nested" ]; then
                $SUDO_CMD mv "$nested" "$INSTALL_DIR/bin/zenobox"
                $SUDO_CMD rm -rf "$(dirname "$nested")"
            fi
        done

        rm -f "$TARBALL_FILE" "$TARBALL_MUSL" "$TARBALL_GNU" 2>/dev/null
        $SUDO_CMD chmod +x "$INSTALL_DIR/bin/zenobox"
        log_success "Zenobox binary downloaded and installed successfully from GitHub Releases."
    else
        log_error "Could not download remote binary for ${VERSION} from GitHub Releases."
        exit 1
    fi
fi

# 4. Create Docker & Docker-Compose Symlinks
log_info "Configuring system symlinks..."

SUDO_CMD=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
fi

for TARGET_DIR in "/usr/local/bin" "/usr/bin"; do
    if [ -d "$TARGET_DIR" ]; then
        $SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$TARGET_DIR/zenobox" 2>/dev/null
        $SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$TARGET_DIR/docker" 2>/dev/null
        $SUDO_CMD ln -sf "$INSTALL_DIR/bin/zenobox" "$TARGET_DIR/docker-compose" 2>/dev/null
    fi
done

if [ ! -f "/usr/bin/docker" ] && [ ! -f "/usr/local/bin/docker" ]; then
    mkdir -p "$HOME/.local/bin"
    ln -sf "$INSTALL_DIR/bin/zenobox" "$HOME/.local/bin/zenobox"
    ln -sf "$INSTALL_DIR/bin/zenobox" "$HOME/.local/bin/docker"
    ln -sf "$INSTALL_DIR/bin/zenobox" "$HOME/.local/bin/docker-compose"
fi

log_success "Symlinks configured successfully in system PATH locations."

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

# 6. Auto-Detect Distro & Configure Docker Daemon Service (1Panel Compatible)
log_info "Detecting Linux distribution and service manager..."

DISTRO_INFO="Unknown Linux"
if [ -f /etc/os-release ]; then
    DISTRO_INFO="$(grep -E '^PRETTY_NAME=' /etc/os-release | cut -d= -f2 | tr -d '"')"
fi
log_info "Detected OS: ${BOLD}${DISTRO_INFO}${NC}"

if command -v systemctl >/dev/null 2>&1 && [ -d /etc/systemd/system ]; then
    log_info "Configuring Systemd service 'docker.service' for 1Panel compatibility..."

    SERVICE_FILE="/etc/systemd/system/docker.service"
    SERVICE_CONTENT="[Unit]
Description=Zenobox Container Engine (Docker Compatible Daemon)
After=network.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/bin/zenobox daemon --port 2375
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
Alias=docker.service"

    echo "$SERVICE_CONTENT" | $SUDO_CMD tee "$SERVICE_FILE" >/dev/null
    $SUDO_CMD systemctl daemon-reload 2>/dev/null
    $SUDO_CMD systemctl enable docker 2>/dev/null
    $SUDO_CMD systemctl restart docker 2>/dev/null || $SUDO_CMD systemctl start docker 2>/dev/null
    log_success "Systemd 'docker.service' registered and activated on port 2375."

elif command -v rc-service >/dev/null 2>&1 || [ -d /etc/init.d ]; then
    log_info "Configuring OpenRC/Init service 'docker'..."

    INIT_FILE="/etc/init.d/docker"
    INIT_CONTENT="#!/sbin/openrc-run
description=\"Zenobox Container Engine Daemon\"
command=\"${INSTALL_DIR}/bin/zenobox\"
command_args=\"daemon --port 2375\"
command_background=\"yes\"
pidfile=\"/run/zenobox.pid\""

    echo "$INIT_CONTENT" | $SUDO_CMD tee "$INIT_FILE" >/dev/null
    $SUDO_CMD chmod +x "$INIT_FILE"
    if command -v rc-update >/dev/null 2>&1; then
        $SUDO_CMD rc-update add docker default 2>/dev/null
        $SUDO_CMD rc-service docker start 2>/dev/null
    else
        $SUDO_CMD service docker start 2>/dev/null
    fi
    log_success "OpenRC/Init 'docker' service registered and activated."
else
    log_warn "No supported service manager (Systemd/OpenRC) detected."
    log_warn "To run daemon manually: ${INSTALL_DIR}/bin/zenobox daemon --port 2375"
fi

# 7. Verification Check
echo ""
log_info "Verifying Zenobox installation..."
if command -v zenobox >/dev/null 2>&1; then
    VER_OUT="$(zenobox --version 2>&1)"
    log_success "Zenobox is active! (${VER_OUT})"
else
    log_warn "Zenobox installed at $INSTALL_DIR/bin/zenobox. Ensure $SYMLINK_DIR is in your PATH."
fi

# 8. Complete Notice
echo -e "\n=================================================="
log_success "Zenobox Installation Completed!"
echo -e "=================================================="
echo -e "${BOLD}Quick Commands:${NC}"
echo -e "  - Test Docker replacement : ${CYAN}docker --help${NC}"
echo -e "  - Check Docker Daemon API : ${CYAN}curl http://localhost:2375/v1.41/_ping${NC}"
echo -e "  - Check Systemd Service   : ${CYAN}systemctl status docker${NC}"
echo -e "  - Pull container image    : ${CYAN}docker pull alpine${NC}"
echo -e "  - Run container           : ${CYAN}docker run -d -p 8080:80 --name my-nginx nginx:alpine${NC}"
echo -e "=================================================="
