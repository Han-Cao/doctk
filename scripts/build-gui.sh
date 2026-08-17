#!/usr/bin/env bash
# Build the Tauri GUI for the current OS.
#
# Frontend assets are written to build/gui-dist by vite.config.ts.
# Rust artifacts and Tauri bundles are written to build/cargo-target via
# CARGO_TARGET_DIR (the Tauri CLI invokes Cargo, which honors this env var).
#
# Prerequisites (Linux): libwebkit2gtk-4.1-dev libgtk-3-dev
# libayatana-appindicator3-dev librsvg2-dev libssl-dev libdbus-1-dev pkg-config
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$ROOT/build"
TARGET_DIR="$BUILD_DIR/cargo-target"
export CARGO_TARGET_DIR="$TARGET_DIR"

cd "$ROOT/gui"
echo "==> Building Tauri GUI (release)"
npm run tauri:build

echo "==> GUI bundles (if any) are under: $TARGET_DIR/release/bundle"
