# dmaf engine installer (Windows PowerShell)
# Official entry: https://dmaf.aicoffe.com/install.ps1
# Contract: fixed install location, re-running the same command upgrades in place; user-registry PATH write; download sha256 verified fail-closed.
# PATH policy (D9): user-registry write via [Environment]::SetEnvironmentVariable; NEVER setx (1024-char truncation trap).
# Fixed install location; re-run = upgrade (idempotent, non-interactive); download sha256 verified fail-closed.
# NOTE: keep this file pure ASCII -- PowerShell 5.1 reads BOM-less UTF-8 as ANSI/GBK and Chinese comments break parsing.
param(
  [string]$Version = "",
  [ValidateSet("auto", "market", "github")]
  [string]$Source = "auto",
  [switch]$NoPath,
  [switch]$Force
)

$ErrorActionPreference = "Stop"
$MarketBase = if ($env:DMAF_MARKET_URL) { $env:DMAF_MARKET_URL } else { "https://dmaf.aicoffe.com" }
$RepoSlug = "aicoffe/dmaf-engine"
$InstallDir = if ($env:DMAF_INSTALL_DIR) { $env:DMAF_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\dmaf" }
$Target = "x86_64-pc-windows-msvc"

if ($env:PROCESSOR_ARCHITECTURE -ne "AMD64") {
    Write-Error "install.ps1: unsupported architecture '$($env:PROCESSOR_ARCHITECTURE)' (only x86_64 is shipped for Windows)"
}

$ProgressPreference = "SilentlyContinue"   # IWR progress bar slows downloads badly on PS 5.1

function Get-LatestVersion {
    param([string]$Source)
    if ($Source -ne "github") {
        try {
            $m = Invoke-RestMethod -Uri "$MarketBase/api/engine/latest" -TimeoutSec 15
            if ($m.version) { return ($m.version -replace "^v", "") }
        } catch {}
    }
    if ($Source -ne "market") {
        $r = Invoke-RestMethod -Uri "https://api.github.com/repos/$RepoSlug/releases/latest" `
            -Headers @{ "User-Agent" = "dmaf-installer" } -TimeoutSec 15
        return ($r.tag_name -replace "^v", "")
    }
    throw "install.ps1: cannot resolve latest version (source=$Source)"
}

if (-not $Version) { $Version = Get-LatestVersion -Source $Source }
$Version = $Version -replace "^v", ""

# Naming frozen to dist 0.34 output (plan section 6.1): archive = dmaf-engine-<target>.zip (no version segment;
# version lives in the release tag), checksum = same-name .sha256 ("<hash>  <filename>").
$Asset = "dmaf-engine-$Target.zip"
switch ($Source) {
    "market" { $BaseUrl = "$MarketBase/engine/v$Version" }
    "github" { $BaseUrl = "https://github.com/$RepoSlug/releases/download/v$Version" }
    "auto" {
        $BaseUrl = "$MarketBase/engine/v$Version"
        try {
            Invoke-WebRequest -Uri "$BaseUrl/$Asset" -Method Head -UseBasicParsing -TimeoutSec 15 | Out-Null
        } catch {
            Write-Host "==> market unreachable, falling back to GitHub"
            $BaseUrl = "https://github.com/$RepoSlug/releases/download/v$Version"
        }
    }
}

$Tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "dmaf-install-$([guid]::NewGuid().ToString('N'))")
try {
    Write-Host "==> downloading $Asset (v$Version, $Source)"
    Invoke-WebRequest -Uri "$BaseUrl/$Asset" -OutFile (Join-Path $Tmp $Asset) -UseBasicParsing
    Invoke-WebRequest -Uri "$BaseUrl/$Asset.sha256" -OutFile (Join-Path $Tmp "$Asset.sha256") -UseBasicParsing

    $WantHash = ((Get-Content (Join-Path $Tmp "$Asset.sha256") | Select-Object -First 1) -split "\s+")[0]
    if (-not $WantHash) { throw "install.ps1: $Asset.sha256 missing or malformed (fail closed)" }
    $GotHash = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $Tmp $Asset)).Hash.ToLower()
    if ($GotHash -ne $WantHash.ToLower()) {
        throw "install.ps1: sha256 mismatch (fail closed)`n  want $WantHash`n  got  $GotHash"
    }

    Expand-Archive -Path (Join-Path $Tmp $Asset) -DestinationPath $Tmp -Force
    $Binary = Join-Path $Tmp "dmaf.exe"
    if (-not (Test-Path $Binary)) {
        # dist may nest archive contents one level deep in <archive-basename>/ - accept both shapes
        $Nested = Get-ChildItem -Path $Tmp -Directory | ForEach-Object { Join-Path $_.FullName "dmaf.exe" } | Where-Object { Test-Path $_ } | Select-Object -First 1
        if ($Nested) { $Binary = $Nested } else { throw "install.ps1: archive does not contain dmaf.exe" }
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    # Fixed location overwrite = upgrade (idempotent; -Force kept as explicit alias, installer never prompts)
    Copy-Item -Path $Binary -Destination (Join-Path $InstallDir "dmaf.exe") -Force

    if (-not $NoPath) {
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if (($UserPath -split ";") -notcontains $InstallDir) {
            $NewPath = ($UserPath.TrimEnd(";") + ";" + $InstallDir)
            [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
            Write-Host "==> PATH updated (user registry): $InstallDir (restart your terminal to take effect)"
        }
    }

    Write-Host "==> installed: $(Join-Path $InstallDir 'dmaf.exe')"
    & (Join-Path $InstallDir "dmaf.exe") --version
    Write-Host "==> next: run 'dmaf doctor' to verify the environment"
    Write-Host "    workflow project: dmaf add https://dmaf.aicoffe.com/aicoffe/dmaf-sdlc"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
