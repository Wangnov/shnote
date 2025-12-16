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

# Language detection
detect_lang() {
    # Check environment variables in order
    for var in SHNOTE_LANG LC_ALL LC_MESSAGES LANGUAGE LANG; do
        eval "val=\$$var"
        if [ -n "$val" ]; then
            # For LANGUAGE, take only the first entry (before colon)
            if [ "$var" = "LANGUAGE" ]; then
                val=$(echo "$val" | cut -d: -f1)
            fi
            # Remove .UTF-8 suffix and convert to lowercase
            val=$(echo "$val" | sed 's/\..*$//' | tr '[:upper:]' '[:lower:]')
            # Skip C/POSIX
            case "$val" in
                c|posix) continue ;;
            esac
            # Check for zh or en
            case "$val" in
                zh*) echo "zh"; return ;;
                en*) echo "en"; return ;;
            esac
        fi
    done

    # macOS: try AppleLocale
    if [ "$(uname -s)" = "Darwin" ]; then
        locale=$(defaults read -g AppleLocale 2>/dev/null || true)
        if [ -n "$locale" ]; then
            locale=$(echo "$locale" | tr '[:upper:]' '[:lower:]')
            case "$locale" in
                zh*) echo "zh"; return ;;
                en*) echo "en"; return ;;
            esac
        fi
    fi

    # Default to English
    echo "en"
}

LANG_CODE=$(detect_lang)

# i18n messages
msg() {
    local key="$1"
    shift
    case "$LANG_CODE" in
        zh)
            case "$key" in
                info_detected) printf "检测到：%s %s (%s)" "$1" "$2" "$3" ;;
                info_proxy) printf "使用 GitHub 代理：%s" "$1" ;;
                info_downloading) printf "正在下载 shnote %s (%s)..." "$1" "$2" ;;
                info_verifying) printf "正在校验..." ;;
                info_installing) printf "正在安装到 %s..." "$1" ;;
                info_success) printf "shnote %s 安装成功！" "$1" ;;
                info_path_exists) printf "安装目录已在 PATH 中" ;;
                info_path_configured) printf "PATH 已在 %s 中配置" "$1" ;;
                info_adding_path) printf "正在将 %s 添加到 %s 中的 PATH" "$1" "$2" ;;
                info_path_done) printf "PATH 配置成功" ;;
                info_run_help) printf "运行 'shnote --help' 开始使用" ;;
                warn_no_shell_config) printf "无法检测 shell 配置文件" ;;
                warn_restart) printf "请重启终端或运行：" ;;
                err_unsupported_os) printf "不支持的操作系统：%s" "$1" ;;
                err_unsupported_arch) printf "不支持的架构：%s" "$1" ;;
                err_no_curl_wget) printf "未找到 curl 或 wget，请先安装" ;;
                err_version_failed) printf "获取最新版本失败" ;;
                err_download_failed) printf "下载失败" ;;
                err_checksum_failed) printf "校验失败！\n预期：%s\n实际：%s" "$1" "$2" ;;
                hint_add_path) printf "请手动将以下内容添加到 shell 配置文件：" ;;
            esac
            ;;
        *)
            case "$key" in
                info_detected) printf "Detected: %s %s (%s)" "$1" "$2" "$3" ;;
                info_proxy) printf "Using GitHub proxy: %s" "$1" ;;
                info_downloading) printf "Downloading shnote %s for %s..." "$1" "$2" ;;
                info_verifying) printf "Verifying checksum..." ;;
                info_installing) printf "Installing to %s..." "$1" ;;
                info_success) printf "shnote %s installed successfully!" "$1" ;;
                info_path_exists) printf "Installation directory is already in PATH" ;;
                info_path_configured) printf "PATH already configured in %s" "$1" ;;
                info_adding_path) printf "Adding %s to PATH in %s" "$1" "$2" ;;
                info_path_done) printf "PATH configured successfully" ;;
                info_run_help) printf "Run 'shnote --help' to get started" ;;
                warn_no_shell_config) printf "Could not detect shell config file" ;;
                warn_restart) printf "Please restart your terminal or run:" ;;
                err_unsupported_os) printf "Unsupported OS: %s" "$1" ;;
                err_unsupported_arch) printf "Unsupported architecture: %s" "$1" ;;
                err_no_curl_wget) printf "Neither curl nor wget found. Please install one of them." ;;
                err_version_failed) printf "Failed to get latest version" ;;
                err_download_failed) printf "Download failed" ;;
                err_checksum_failed) printf "Checksum verification failed!\nExpected: %s\nActual:   %s" "$1" "$2" ;;
                hint_add_path) printf "Please add the following to your shell profile manually:" ;;
            esac
            ;;
    esac
}

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
        *)       error "$(msg err_unsupported_os "$(uname -s)")" ;;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        arm64|aarch64)  echo "aarch64" ;;
        *)              error "$(msg err_unsupported_arch "$(uname -m)")" ;;
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

# Get latest version from GitHub (always direct, proxy doesn't support API)
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$api_url" | \
            grep '"tag_name":' | \
            sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$api_url" | \
            grep '"tag_name":' | \
            sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "$(msg err_no_curl_wget)"
    fi
}

# Download file
download_file() {
    local url="$1"
    local output="$2"

    if command -v curl >/dev/null 2>&1; then
        curl -fL --progress-bar -o "$output" "$url"
    elif command -v wget >/dev/null 2>&1; then
        wget --show-progress -q -O "$output" "$url"
    else
        error "$(msg err_no_curl_wget)"
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
        error "$(msg err_checksum_failed "$expected" "$actual")"
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

    info "$(msg info_downloading "$version" "$target")"
    download_file "$url" "${tmp_dir}/shnote" || error "$(msg err_download_failed)"
    download_file "$checksum_url" "${tmp_dir}/shnote.sha256" || error "$(msg err_download_failed)"

    info "$(msg info_verifying)"
    expected_hash=$(awk '{print $1}' "${tmp_dir}/shnote.sha256")
    verify_checksum "${tmp_dir}/shnote" "$expected_hash"

    info "$(msg info_installing "$INSTALL_DIR")"
    mkdir -p "$INSTALL_DIR"
    chmod +x "${tmp_dir}/shnote"
    mv "${tmp_dir}/shnote" "${INSTALL_DIR}/shnote"

    info "$(msg info_success "$version")"
}

# Check if in PATH and add if not
setup_path() {
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "$(msg info_path_exists)"
            return 0
            ;;
    esac

    # Detect shell and config file
    local shell_name
    local config_file=""

    shell_name=$(basename "$SHELL")

    case "$shell_name" in
        bash)
            if [ -f "$HOME/.bashrc" ]; then
                config_file="$HOME/.bashrc"
            elif [ -f "$HOME/.bash_profile" ]; then
                config_file="$HOME/.bash_profile"
            fi
            ;;
        zsh)
            config_file="$HOME/.zshrc"
            ;;
        fish)
            config_file="$HOME/.config/fish/config.fish"
            ;;
    esac

    if [ -z "$config_file" ]; then
        warn "$(msg warn_no_shell_config)"
        echo ""
        echo "$(msg hint_add_path)"
        echo ""
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
        return 0
    fi

    # Check if already configured in the file
    if [ -f "$config_file" ] && grep -q "$INSTALL_DIR" "$config_file" 2>/dev/null; then
        info "$(msg info_path_configured "$config_file")"
        return 0
    fi

    # Add to config file
    info "$(msg info_adding_path "$INSTALL_DIR" "$config_file")"

    if [ "$shell_name" = "fish" ]; then
        mkdir -p "$(dirname "$config_file")"
        echo "" >> "$config_file"
        echo "# Added by shnote installer" >> "$config_file"
        echo "fish_add_path $INSTALL_DIR" >> "$config_file"
    else
        echo "" >> "$config_file"
        echo "# Added by shnote installer" >> "$config_file"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$config_file"
    fi

    info "$(msg info_path_done)"
    echo ""
    warn "$(msg warn_restart)"
    echo ""
    echo "  source $config_file"
    echo ""
}

main() {
    local os
    local arch
    local target

    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "$os" "$arch")

    info "$(msg info_detected "$os" "$arch" "$target")"

    # Show proxy info if configured
    if [ -n "$GITHUB_PROXY" ]; then
        info "$(msg info_proxy "$GITHUB_PROXY")"
    fi

    local version="${SHNOTE_VERSION:-$(get_latest_version)}"
    [ -z "$version" ] && error "$(msg err_version_failed)"

    download_and_install "$version" "$target"
    setup_path

    echo ""
    info "$(msg info_run_help)"
}

main "$@"
