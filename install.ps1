# Navim - Terminal Web Browser Installer for Windows
# https://github.com/politikl/navim

$ErrorActionPreference = "Stop"

$Repo = "politikl/navim"
$InstallDir = "$env:USERPROFILE\.local\bin"

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
$Target = "$Arch-pc-windows-msvc"

Write-Host "Detected: Windows $Arch"
Write-Host "Installing navim for $Target..."

# Get latest release
$LatestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
$Latest = $LatestRelease.tag_name

if (-not $Latest) {
    Write-Host "Failed to fetch latest release"
    exit 1
}

Write-Host "Latest version: $Latest"

# Download binary
$DownloadUrl = "https://github.com/$Repo/releases/download/$Latest/navim-$Target.exe"
Write-Host "Downloading from: $DownloadUrl"

# Create install directory
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Download
$OutputPath = "$InstallDir\navim.exe"
Invoke-WebRequest -Uri $DownloadUrl -OutFile $OutputPath

Write-Host ""
Write-Host "Navim installed to $OutputPath" -ForegroundColor Green
Write-Host ""
Write-Host "Add to your PATH:"
Write-Host ""
Write-Host "1. Press Win + X, select 'System'"
Write-Host "2. Click 'Advanced system settings'"
Write-Host "3. Click 'Environment Variables'"
Write-Host "4. Under 'User variables', select 'Path' and click 'Edit'"
Write-Host "5. Click 'New' and add: $InstallDir"
Write-Host "6. Click OK and restart your terminal"
Write-Host ""
Write-Host "Or run this command in PowerShell (as Administrator):"
Write-Host ""
Write-Host "    [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$InstallDir', 'User')"
Write-Host ""
Write-Host "Usage: navim <query>"
Write-Host "       navim -h     (view history)"
Write-Host "       navim about  (about info)"
