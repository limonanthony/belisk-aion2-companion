param(
    [ValidateSet("debug", "release")]
    [string]$Profile = "debug",
    # Where Cargo puts the build, relative to the project root. The Tauri app
    # builds in src-tauri\target; the workspace (crates/app) in target.
    [string]$TargetRoot = "src-tauri\target"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$sourceDir = Join-Path $projectRoot "src-tauri\resources\windivert"
$targetDir = Join-Path $projectRoot "$TargetRoot\$Profile"

New-Item -ItemType Directory -Path $targetDir -Force | Out-Null

function Get-Sha256([string]$Path) {
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [Convert]::ToBase64String($sha256.ComputeHash([System.IO.File]::ReadAllBytes($Path)))
    }
    finally {
        $sha256.Dispose()
    }
}

foreach ($fileName in @("WinDivert.dll", "WinDivert64.sys")) {
    $source = Join-Path $sourceDir $fileName
    $destination = Join-Path $targetDir $fileName

    if ((Test-Path $destination) -and
        ((Get-Sha256 $source) -eq (Get-Sha256 $destination))) {
        continue
    }

    Copy-Item -LiteralPath $source -Destination $destination -Force
}
