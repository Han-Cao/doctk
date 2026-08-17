# Build the doctk CLI and copy the binary into build\bin.
# All compilation artifacts stay under the root-level build\ directory.
$ErrorActionPreference = "Stop"

$ROOT = Resolve-Path (Join-Path $PSScriptRoot "..")
$BUILD_DIR = Join-Path $ROOT "build"
$TARGET_DIR = Join-Path $BUILD_DIR "cargo-target"
$BIN_DIR = Join-Path $BUILD_DIR "bin"

Write-Host "==> Building doctk CLI (release)"
$env:CARGO_TARGET_DIR = $TARGET_DIR
cargo build --release -p doctk-cli --manifest-path (Join-Path $ROOT "Cargo.toml")

New-Item -ItemType Directory -Force -Path $BIN_DIR | Out-Null
Copy-Item (Join-Path $TARGET_DIR "release\doctk.exe") (Join-Path $BIN_DIR "doctk.exe") -Force
Write-Host "==> Built: $(Join-Path $BIN_DIR 'doctk.exe')"
