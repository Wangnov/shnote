#!/bin/sh
# shnote installer for Unix-like systems
# Usage: curl -fsSL https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.sh | sh
#
# Environment variables:
#   SHNOTE_INSTALL_DIR  - Installation directory (default: ~/.local/bin)
#   SHNOTE_VERSION      - Specific version to install (default: latest)
#   GITHUB_PROXY        - GitHub mirror/proxy URL for faster downloads in China
#                         Find available proxies at: https://ghproxylist.com/
#
# Example with GitHub proxy (for users in China):
#   GITHUB_PROXY=https://ghfast.top curl -fsSL ... | sh

set -e

# Configuration
REPO="wangnov/shnote"
INSTALL_DIR="${SHNOTE_INSTALL_DIR:-$HOME/.local/bin}"
# GitHub proxy for accelerating downloads (e.g., https://ghproxy.top)
GITHUB_PROXY="${GITHUB_PROXY:-}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1" >&2
    exit 1
}

# Apply GitHub proxy to URL if configured
proxy_url() {
    local url="$1"
    if [ -n "$GITHUB_PROXY" ]; then
        # Remove trailing slash from proxy URL
        local proxy="${GITHUB_PROXY%/}"
        echo "${proxy}/${url}"
    else
        echo "$url"
    fi
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)       error "Unsupported OS: $(uname -s)" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        arm64|aarch64)  echo "aarch64" ;;
        *)              error "Unsupported architecture: $(uname -m)" ;;
    esac
}

# Get target triple
get_target() {
    local os="$1"
    local arch="$2"

    case "$os" in
        macos)
            echo "${arch}-apple-darwin"
            ;;
        linux)
            echo "${arch}-unknown-linux-musl"
            ;;
    esac
}

# Get latest version from GitHub
get_latest_version() {
    local api_url
    api_url=$(proxy_url "https://api.github.com/repos/${REPO}/releases/latest")

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api_url" | \
            grep '"tag_name":' | \
            sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" | \
            grep '"tag_name":' | \
            sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download file
download_file() {
    local url="$1"
    local output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$output" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$output" "$url"
    else
        error "Neither curl nor wget found."
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local expected="$2"

    if command -v sha256sum >/dev/null 2>&1; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "No checksum tool found, skipping verification"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed!\nExpected: $expected\nActual:   $actual"
    fi
}

# Download and install
download_and_install() {
    local version="$1"
    local target="$2"
    local artifact="shnote-${target}"
    local base_url="https://github.com/${REPO}/releases/download/${version}/${artifact}"
    local url
    local checksum_url

    url=$(proxy_url "$base_url")
    checksum_url=$(proxy_url "${base_url}.sha256")

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    info "Downloading shnote ${version} for ${target}..."
    download_file "$url" "${tmp_dir}/shnote" || error "Failed to download binary"
    download_file "$checksum_url" "${tmp_dir}/shnote.sha256" || error "Failed to download checksum"

    info "Verifying checksum..."
    expected_hash=$(awk '{print $1}' "${tmp_dir}/shnote.sha256")
    verify_checksum "${tmp_dir}/shnote" "$expected_hash"

    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    chmod +x "${tmp_dir}/shnote"
    mv "${tmp_dir}/shnote" "${INSTALL_DIR}/shnote"

    info "shnote ${version} installed successfully!"
}

# Check if in PATH
check_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            return 0
            ;;
        *)
            warn "Installation directory is not in PATH"
            echo ""
            echo "Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo ""
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
            echo ""
            ;;
    esac
}

main() {
    local os
    local arch
    local target

    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "$os" "$arch")

    info "Detected: ${os} ${arch} (${target})"

    # Show proxy info if configured
    if [ -n "$GITHUB_PROXY" ]; then
        info "Using GitHub proxy: ${GITHUB_PROXY}"
    fi

    local version="${SHNOTE_VERSION:-$(get_latest_version)}"
    [ -z "$version" ] && error "Failed to get latest version"

    download_and_install "$version" "$target"
    check_path

    echo ""
    info "Run 'shnote --help' to get started"
}

main "$@"
