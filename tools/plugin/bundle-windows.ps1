
$ErrorActionPreference = "Stop"

$Root = Resolve-Path "$PSScriptRoot/../.."

$Dll = Join-Path $Root "target/release/chimera_plugin.dll"
$Output = Join-Path $Root "dist"

Write-Host "Packaging Chimera Lab for Windows..."

if (!(Test-Path $Dll)) {
    throw "Compiled Windows DLL not found: $Dll"
}

New-Item -ItemType Directory -Force -Path $Output | Out-Null

$Plugin = Join-Path $Output "Chimera Lab.vst3"

Copy-Item $Dll $Plugin -Force

Write-Host "Windows VST3 created:"
Write-Host $Plugin
