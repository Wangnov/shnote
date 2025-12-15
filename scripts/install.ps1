# shnote installer for Windows
# Usage: irm https://raw.githubusercontent.com/wangnov/shnote/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

# Configuration
$Repo = "wangnov/shnote"
$InstallDir = if ($env:SHNOTE_INSTALL_DIR) { $env:SHNOTE_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

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

# Get latest version from GitHub
function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        return $response.tag_name
    }
    catch {
        Write-Err "Failed to get latest version: $_"
    }
}

# Download and install
function Install-Shnote {
    param([string]$Version)

    $Target = "x86_64-pc-windows-msvc"
    $Artifact = "shnote-$Target.exe"
    $Url = "https://github.com/$Repo/releases/download/$Version/$Artifact"
    $ChecksumUrl = "$Url.sha256"

    $TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TempDir | Out-Null

    try {
        Write-Info "Downloading shnote $Version for Windows..."
        Invoke-WebRequest -Uri $Url -OutFile "$TempDir\shnote.exe"
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile "$TempDir\shnote.exe.sha256"

        Write-Info "Verifying checksum..."
        $ExpectedHash = (Get-Content "$TempDir\shnote.exe.sha256").Split()[0].Trim().ToLower()
        $ActualHash = (Get-FileHash "$TempDir\shnote.exe" -Algorithm SHA256).Hash.ToLower()

        if ($ExpectedHash -ne $ActualHash) {
            Write-Err "Checksum verification failed!`nExpected: $ExpectedHash`nActual:   $ActualHash"
        }

        Write-Info "Installing to $InstallDir..."
        if (!(Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir | Out-Null
        }
        Move-Item -Force "$TempDir\shnote.exe" "$InstallDir\shnote.exe"

        Write-Info "shnote $Version installed successfully!"
    }
    finally {
        Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    }
}

# Check if install directory is in PATH
function Test-InPath {
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Warn "Installation directory is not in PATH"
        Write-Host ""
        Write-Host "To add it permanently, run:"
        Write-Host ""
        Write-Host "  `$path = [Environment]::GetEnvironmentVariable('Path', 'User')"
        Write-Host "  [Environment]::SetEnvironmentVariable('Path', `"`$path;$InstallDir`", 'User')"
        Write-Host ""
        Write-Host "Then restart your terminal."
    }
}

# Main
Write-Info "Detected: Windows x86_64"

$Version = if ($env:SHNOTE_VERSION) { $env:SHNOTE_VERSION } else { Get-LatestVersion }
if (!$Version) {
    Write-Err "Failed to get latest version"
}

Install-Shnote -Version $Version
Test-InPath

Write-Host ""
Write-Info "Run 'shnote --help' to get started"
