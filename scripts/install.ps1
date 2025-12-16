# shnote installer for Windows
# Usage: irm https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.ps1 | iex
#
# Environment variables:
#   SHNOTE_INSTALL_DIR  - Installation directory (default: ~/.local/bin)
#   SHNOTE_VERSION      - Specific version to install (default: latest)
#   GITHUB_PROXY        - GitHub mirror/proxy URL for faster downloads in China
#                         Find available proxies at: https://ghproxylist.com/
#
# Example with GitHub proxy (for users in China):
#   $env:GITHUB_PROXY = "https://ghfast.top"; irm ... | iex

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "wangnov/shnote"
$InstallDir = if ($env:SHNOTE_INSTALL_DIR) { $env:SHNOTE_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }
$GitHubProxy = if ($env:GITHUB_PROXY) { $env:GITHUB_PROXY.TrimEnd('/') } else { "" }

# Language detection
function Get-Lang {
    # Check environment variables
    $envVars = @("SHNOTE_LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG")
    foreach ($var in $envVars) {
        $val = [Environment]::GetEnvironmentVariable($var)
        if ($val) {
            # For LANGUAGE, take only the first entry
            if ($var -eq "LANGUAGE") {
                $val = $val.Split(':')[0]
            }
            # Remove .UTF-8 suffix
            $val = $val -replace '\..*$', ''
            $val = $val.ToLower()
            # Skip C/POSIX
            if ($val -eq "c" -or $val -eq "posix") { continue }
            # Check for zh or en
            if ($val.StartsWith("zh")) { return "zh" }
            if ($val.StartsWith("en")) { return "en" }
        }
    }

    # Windows: try Get-Culture
    try {
        $culture = (Get-Culture).Name.ToLower()
        if ($culture.StartsWith("zh")) { return "zh" }
        if ($culture.StartsWith("en")) { return "en" }
    } catch {}

    return "en"
}

$LangCode = Get-Lang

# i18n messages
function Get-Msg($key, $p1 = "", $p2 = "", $p3 = "") {
    if ($LangCode -eq "zh") {
        switch ($key) {
            "info_detected" { "检测到：Windows x86_64" }
            "info_proxy" { "使用 GitHub 代理：$p1" }
            "info_downloading" { "正在下载 shnote $p1 (Windows)..." }
            "info_verifying" { "正在校验..." }
            "info_installing" { "正在安装到 $p1..." }
            "info_success" { "shnote $p1 安装成功！" }
            "info_path_exists" { "安装目录已在 PATH 中" }
            "info_adding_path" { "正在将 $p1 添加到用户 PATH..." }
            "info_path_done" { "PATH 配置成功" }
            "info_run_help" { "运行 'shnote --help' 开始使用" }
            "warn_restart" { "请重启终端使更改完全生效" }
            "err_version_failed" { "获取最新版本失败" }
            "err_checksum_failed" { "校验失败！`n预期：$p1`n实际：$p2" }
            default { $key }
        }
    } else {
        switch ($key) {
            "info_detected" { "Detected: Windows x86_64" }
            "info_proxy" { "Using GitHub proxy: $p1" }
            "info_downloading" { "Downloading shnote $p1 for Windows..." }
            "info_verifying" { "Verifying checksum..." }
            "info_installing" { "Installing to $p1..." }
            "info_success" { "shnote $p1 installed successfully!" }
            "info_path_exists" { "Installation directory is already in PATH" }
            "info_adding_path" { "Adding $p1 to user PATH..." }
            "info_path_done" { "PATH configured successfully" }
            "info_run_help" { "Run 'shnote --help' to get started" }
            "warn_restart" { "Please restart your terminal for changes to take full effect" }
            "err_version_failed" { "Failed to get latest version" }
            "err_checksum_failed" { "Checksum verification failed!`nExpected: $p1`nActual:   $p2" }
            default { $key }
        }
    }
}

function Write-Info($msg) {
    Write-Host "[INFO] $msg" -ForegroundColor Green
}

function Write-Warn($msg) {
    Write-Host "[WARN] $msg" -ForegroundColor Yellow
}

function Write-Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
    exit 1
}

# Apply GitHub proxy to URL if configured
function Get-ProxiedUrl($url) {
    if ($GitHubProxy) {
        return "$GitHubProxy/$url"
    }
    return $url
}

# Get latest version from GitHub (always direct, proxy doesn't support API)
function Get-LatestVersion {
    try {
        $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $response = Invoke-RestMethod -Uri $apiUrl
        return $response.tag_name
    }
    catch {
        Write-Err (Get-Msg "err_version_failed")
    }
}

# Download and install
function Install-Shnote {
    param([string]$Version)

    $Target = "x86_64-pc-windows-msvc"
    $Artifact = "shnote-$Target.exe"
    $BaseUrl = "https://github.com/$Repo/releases/download/$Version/$Artifact"
    $Url = Get-ProxiedUrl $BaseUrl
    $ChecksumUrl = Get-ProxiedUrl "$BaseUrl.sha256"

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    try {
        Write-Info (Get-Msg "info_downloading" $Version)
        Invoke-WebRequest -Uri $Url -OutFile "$TempDir\shnote.exe"
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile "$TempDir\shnote.exe.sha256"

        Write-Info (Get-Msg "info_verifying")
        $ExpectedHash = (Get-Content "$TempDir\shnote.exe.sha256").Split()[0].Trim().ToLower()
        $ActualHash = (Get-FileHash "$TempDir\shnote.exe" -Algorithm SHA256).Hash.ToLower()

        if ($ExpectedHash -ne $ActualHash) {
            Write-Err (Get-Msg "err_checksum_failed" $ExpectedHash $ActualHash)
        }

        Write-Info (Get-Msg "info_installing" $InstallDir)
        if (!(Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir | Out-Null
        }
        Move-Item -Force "$TempDir\shnote.exe" "$InstallDir\shnote.exe"

        Write-Info (Get-Msg "info_success" $Version)
    }
    finally {
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    }
}

# Check if install directory is in PATH and add if not
function Setup-Path {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

    # Check if already in PATH
    if ($UserPath -like "*$InstallDir*") {
        Write-Info (Get-Msg "info_path_exists")
        return
    }

    # Add to user PATH
    Write-Info (Get-Msg "info_adding_path" $InstallDir)

    if ($UserPath) {
        $NewPath = "$UserPath;$InstallDir"
    } else {
        $NewPath = $InstallDir
    }

    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")

    # Also update current session
    $env:Path = "$env:Path;$InstallDir"

    Write-Info (Get-Msg "info_path_done")
    Write-Host ""
    Write-Warn (Get-Msg "warn_restart")
}

# Main
Write-Info (Get-Msg "info_detected")

if ($GitHubProxy) {
    Write-Info (Get-Msg "info_proxy" $GitHubProxy)
}

$Version = if ($env:SHNOTE_VERSION) { $env:SHNOTE_VERSION } else { Get-LatestVersion }
if (!$Version) {
    Write-Err (Get-Msg "err_version_failed")
}

Install-Shnote -Version $Version
Setup-Path

Write-Host ""
Write-Info (Get-Msg "info_run_help")
