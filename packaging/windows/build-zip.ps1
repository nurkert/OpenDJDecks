# Builds the portable Windows release (x86_64) into target/dist:
# OpenDJDecks-<version>-windows-x86_64.zip with the three programs,
# every DLL they need from the build (ONNX Runtime when it is linked
# dynamically), the controller mappings, README and LICENSE.
#
# Requirements: Rust (MSVC toolchain), the Rubber Band library from vcpkg
# (`vcpkg install rubberband:x64-windows`, VCPKG_ROOT set, VCPKGRS_DYNAMIC=1) or
# RUBBERBAND_LIB_DIR pointing at a folder with rubberband.lib.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..\..')

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
cargo build --release --locked -p opendjdecks -p od-cli -p od-mcp
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$name = "OpenDJDecks-$version-windows-x86_64"
$dist = 'target\dist'
$stage = Join-Path $dist $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

foreach ($exe in 'opendjdecks.exe', 'od-cli.exe', 'opendjdecks-mcp.exe') {
    Copy-Item "target\release\$exe" $stage
}
# DLLs next to the executables (ort's copy-dylibs puts ONNX Runtime there
# when it is not linked statically).
# Rubber Band and its dependencies from vcpkg (dynamic: vcpkg has no static
# Rubber Band for Windows).
if ($env:VCPKG_ROOT) {
    $vbin = Join-Path $env:VCPKG_ROOT 'installed\x64-windows\bin'
    if (Test-Path $vbin) {
        Get-ChildItem $vbin -Filter '*.dll' -File | ForEach-Object { Copy-Item $_.FullName $stage }
    }
}
Get-ChildItem 'target\release' -Filter '*.dll' -File | ForEach-Object {
    Copy-Item $_.FullName $stage
}
Copy-Item -Recurse mappings (Join-Path $stage 'mappings')
Copy-Item README.md, LICENSE $stage
Copy-Item packaging\assets\icon-256.png (Join-Path $stage 'opendjdecks.png')

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip
Get-ChildItem $dist
