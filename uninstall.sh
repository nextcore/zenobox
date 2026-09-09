#!/bin/bash

# ==============================================================================
# 🗑️ Zenobox Automated Uninstaller Script
# ==============================================================================
# Script ini menghapus Zenobox OCI Container Runtime, symlink sistem,
# serta membersihkan konfigurasi alias shell dari sistem.
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
echo "          Zenobox Uninstaller System              "
echo "=================================================="
echo -e "${NC}"

DEFAULT_INSTALL_DIR="/opt/zenobox"
INSTALL_DIR="$DEFAULT_INSTALL_DIR"
PURGE_DATA=0

while [ $# -gt 0 ]; do
    case "$1" in
        --dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --purge)
            PURGE_DATA=1
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dir <path>     Tentukan direktori instalasi yang akan dihapus (default: /opt/zenobox)"
            echo "  --purge          Hapus juga data container/image di ~/.zenobox dan /var/lib/zenobox"
            echo "  --help, -h       Tampilkan bantuan ini"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Usage: $0 [--dir <path>] [--purge] [--help|-h]"
            exit 1
            ;;
    esac
done

SUDO_CMD=""
if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
    SUDO_CMD="sudo"
fi

# 1. Stop active containers or daemon if running
if command -v zenobox >/dev/null 2>&1; then
    log_info "Stopping active containers..."
    zenobox ps -a >/dev/null 2>&1
fi

# 2. Remove Installation Directory
if [ -d "$INSTALL_DIR" ]; then
    log_info "Removing installation directory '${INSTALL_DIR}'..."
    $SUDO_CMD rm -rf "$INSTALL_DIR"
    log_success "Directory '${INSTALL_DIR}' removed."
else
    log_warn "Installation directory '${INSTALL_DIR}' not found."
fi

# Remove stray /bin/zenobox if created by mistake
if [ -f "/bin/zenobox" ]; then
    $SUDO_CMD rm -f "/bin/zenobox"
fi

# 3. Remove Symlinks
log_info "Removing system symlinks..."
for LINK_DIR in "/usr/local/bin" "$HOME/.local/bin"; do
    for LINK in "zenobox" "docker" "docker-compose"; do
        TARGET_LINK="${LINK_DIR}/${LINK}"
        if [ -L "$TARGET_LINK" ] || [ -f "$TARGET_LINK" ]; then
            # Verify it's a Zenobox link or force remove if in zenobox install dir
            $SUDO_CMD rm -f "$TARGET_LINK" 2>/dev/null || rm -f "$TARGET_LINK"
            log_success "Removed symlink: ${TARGET_LINK}"
        fi
    done
done

# 4. Remove Shell Aliases
log_info "Cleaning up shell configuration files..."

remove_zenobox_aliases() {
    local rc_file="$1"
    if [ -f "$rc_file" ]; then
        if grep -q "Zenobox Docker Aliases" "$rc_file" || grep -q "alias docker=\"zenobox\"" "$rc_file"; then
            # Use sed to remove Zenobox alias block
            if [ -w "$rc_file" ]; then
                sed -i '/# Zenobox Docker Aliases/d' "$rc_file"
                sed -i '/alias docker="zenobox"/d' "$rc_file"
                sed -i '/alias docker-compose="zenobox compose"/d' "$rc_file"
                log_success "Removed Zenobox aliases from $rc_file"
            fi
        fi
    fi
}

for RC in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.bash_aliases" "$HOME/.profile"; do
    remove_zenobox_aliases "$RC"
done

# 5. Purge Data Directory (Optional)
if [ $PURGE_DATA -eq 1 ]; then
    log_info "Purging Zenobox cached images and container data..."
    rm -rf "$HOME/.zenobox"
    $SUDO_CMD rm -rf "/var/lib/zenobox"
    log_success "Zenobox container data and image cache purged."
else
    log_info "Container image data in ~/.zenobox was preserved."
    log_info "Tip: Run with '--purge' to remove container data as well."
fi

echo -e "\n=================================================="
log_success "Zenobox Uninstallation Completed!"
echo -e "=================================================="
