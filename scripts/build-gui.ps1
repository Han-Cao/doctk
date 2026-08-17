# Build the Tauri GUI on Windows.
#
# Frontend assets are written to build\gui-dist by vite.config.ts.
# Rust artifacts and Tauri bundles are written to build\cargo-target via
# CARGO_TARGET_DIR (the Tauri CLI invokes Cargo, which honors this env var).
#
# Prerequisites: Rust MSVC toolchain, VS Build Tools (Desktop development
# with C++), Node.js, WebView2 Runtime.
$ErrorActionPreference = "Stop"

$ROOT = Resolve-Path (Join-Path $PSScriptRoot "..")
$BUILD_DIR = Join-Path $ROOT "build"
$TARGET_DIR = Join-Path $BUILD_DIR "cargo-target"
$env:CARGO_TARGET_DIR = $TARGET_DIR

Set-Location (Join-Path $ROOT "gui")
Write-Host "==> Building Tauri GUI (release)"
npm run tauri:build

Write-Host "==> GUI bundles (if any) are under: $(Join-Path $TARGET_DIR 'release\bundle')"
