
$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot/../.."

$Dll = Join-Path $Root "target/release/chimera_plugin.dll"

$Output = Join-Path $Root "dist"
$Bundle = Join-Path $Output "Chimera Lab.vst3"

$Architecture = Join-Path $Bundle "Contents/x86_64-win"

Write-Host "Preparing Chimera Lab Windows VST3..."

# Check compiled DLL
if (!(Test-Path $Dll)) {
    throw "Compiled plugin DLL not found: $Dll"
}

# Clean previous build
if (Test-Path $Bundle) {
    Remove-Item $Bundle -Recurse -Force
}

# Create VST3 directories
New-Item -ItemType Directory -Force -Path $Architecture | Out-Null

# Copy plugin DLL
Copy-Item $Dll `
    (Join-Path $Architecture "Chimera Lab.vst3")

# Verify output
$Plugin = Join-Path $Architecture "Chimera Lab.vst3"

if (!(Test-Path $Plugin)) {
    throw "VST3 bundle creation failed!"
}

Write-Host ""
Write-Host "VST3 bundle created successfully!"
Write-Host $Bundle
