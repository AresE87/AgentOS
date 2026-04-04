# AgentOS — Build, Package & Checksum Script
# Usage: powershell -ExecutionPolicy Bypass -File installer/sign_and_package.ps1

param(
    [switch]$SkipBuild,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "  AgentOS Release Builder" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# 1. Build Tauri release
if (-not $SkipBuild) {
    Write-Host "[1/4] Building Tauri release..." -ForegroundColor Yellow
    Write-Host "  Running: cargo tauri build" -ForegroundColor DarkGray

    $buildStart = Get-Date
    cargo tauri build 2>&1 | ForEach-Object {
        if ($Verbose) { Write-Host "  $_" -ForegroundColor DarkGray }
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build FAILED with exit code $LASTEXITCODE" -ForegroundColor Red
        exit 1
    }

    $buildTime = (Get-Date) - $buildStart
    Write-Host "  Build completed in $([math]::Round($buildTime.TotalMinutes, 1)) minutes" -ForegroundColor Green
} else {
    Write-Host "[1/4] Skipping build (--SkipBuild)" -ForegroundColor DarkGray
}

# 2. Locate output artifacts
Write-Host ""
Write-Host "[2/4] Locating build artifacts..." -ForegroundColor Yellow

$bundleDir = "src-tauri\target\release\bundle"

$nsis = Get-ChildItem "$bundleDir\nsis\*.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
$msi  = Get-ChildItem "$bundleDir\msi\*.msi"  -ErrorAction SilentlyContinue | Select-Object -First 1

if ($nsis) {
    $nsisSize = [math]::Round($nsis.Length / 1MB, 1)
    Write-Host "  NSIS Installer: $($nsis.FullName)" -ForegroundColor Green
    Write-Host "    Size: $nsisSize MB" -ForegroundColor DarkGray
} else {
    Write-Host "  NSIS Installer: NOT FOUND" -ForegroundColor Red
}

if ($msi) {
    $msiSize = [math]::Round($msi.Length / 1MB, 1)
    Write-Host "  MSI Installer:  $($msi.FullName)" -ForegroundColor Green
    Write-Host "    Size: $msiSize MB" -ForegroundColor DarkGray
} else {
    Write-Host "  MSI Installer:  NOT FOUND" -ForegroundColor Red
}

if (-not $nsis -and -not $msi) {
    Write-Host ""
    Write-Host "No build artifacts found. Ensure 'cargo tauri build' completed successfully." -ForegroundColor Red
    exit 1
}

# 3. Generate SHA-256 checksums
Write-Host ""
Write-Host "[3/4] Generating SHA-256 checksums..." -ForegroundColor Yellow

$checksumFile = "$bundleDir\checksums.sha256"
$checksums = @()

if ($nsis) {
    $hash = (Get-FileHash $nsis.FullName -Algorithm SHA256).Hash
    $line = "$hash  $($nsis.Name)"
    $checksums += $line
    Write-Host "  $($nsis.Name): $hash" -ForegroundColor DarkGray
}

if ($msi) {
    $hash = (Get-FileHash $msi.FullName -Algorithm SHA256).Hash
    $line = "$hash  $($msi.Name)"
    $checksums += $line
    Write-Host "  $($msi.Name): $hash" -ForegroundColor DarkGray
}

$checksums | Out-File -FilePath $checksumFile -Encoding ASCII
Write-Host "  Checksums saved to: $checksumFile" -ForegroundColor Green

# 4. Summary
Write-Host ""
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host "  Build Complete!" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

if ($nsis) { Write-Host "  NSIS: $($nsis.FullName) ($nsisSize MB)" }
if ($msi)  { Write-Host "  MSI:  $($msi.FullName) ($msiSize MB)" }
Write-Host "  Checksums: $checksumFile"
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "  1. Test the installer on a clean machine" -ForegroundColor DarkGray
Write-Host "  2. Upload to GitHub Releases" -ForegroundColor DarkGray
Write-Host "  3. Update download links in documentation" -ForegroundColor DarkGray
Write-Host ""
