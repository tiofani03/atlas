# Atlas CLI (`atx`) PowerShell Installer for Windows
# Usage: irm https://raw.githubusercontent.com/tiofani03/atlas/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "tiofani03/atlas"
$BinName = "atx.exe"

function Write-Info($msg) {
    Write-Host "[INFO] " -ForegroundColor Cyan -NoNewline
    Write-Host $msg
}

function Write-Success($msg) {
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $msg
}

function Write-Warn($msg) {
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $msg
}

function Write-ErrorMsg($msg) {
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $msg
    exit 1
}

# 1. Architecture Check
if (-not [System.Environment]::Is64BitOperatingSystem) {
    Write-ErrorMsg "Atlas requires a 64-bit Windows operating system."
}

$Target = "x86_64-pc-windows-msvc"
Write-Info "Detected platform: Windows x64 ($Target)"

# 2. Installation Directory ($HOME\.local\bin)
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $HOME ".local\bin" }
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

# 3. Resolve Version
if (-not $env:ATX_VERSION) {
    Write-Info "Resolving latest Atlas release from GitHub..."
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $ReleaseUri = "https://api.github.com/repos/$Repo/releases/latest"
        $Response = Invoke-RestMethod -Uri $ReleaseUri -Headers @{ "Accept" = "application/vnd.github.v3+json" }
        $Version = $Response.tag_name
    } catch {
        Write-ErrorMsg "Failed to query latest release from GitHub: $_"
    }
} else {
    $Version = $env:ATX_VERSION
}

Write-Info "Selected version: $Version"

# 4. Download and Extract Archive
$ArchiveName = "atx-$Version-$Target.zip"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Version/$ArchiveName"

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    $ZipPath = Join-Path $TempDir $ArchiveName
    Write-Info "Downloading $DownloadUrl..."
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath

    Write-Info "Extracting $ArchiveName..."
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    $ExtractedBin = Get-ChildItem -Path $TempDir -Filter $BinName -Recurse | Select-Object -First 1
    if (-not $ExtractedBin) {
        Write-ErrorMsg "Could not find $BinName in the downloaded archive."
    }

    $DestPath = Join-Path $InstallDir $BinName
    Move-Item -Path $ExtractedBin.FullName -Destination $DestPath -Force
    Write-Success "Atlas CLI installed to $DestPath"
} finally {
    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 5. Add to User PATH if not already included
$UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
$PathParts = $UserPath -split ";" | ForEach-Object { $_.TrimEnd("\") }

if ($PathParts -notcontains $InstallDir.TrimEnd("\")) {
    Write-Info "Adding $InstallDir to User PATH..."
    $NewPath = "$UserPath;$InstallDir"
    [System.Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Success "Added $InstallDir to User PATH environment variable."
}

Write-Host ""
Write-Success "Installation complete! Restart your terminal and run:"
Write-Host "  atx init" -ForegroundColor Yellow
