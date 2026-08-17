#!/usr/bin/env bash
# Build the doctk CLI and copy the binary into build/bin.
# All compilation artifacts stay under the root-level build/ directory.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$ROOT/build"
TARGET_DIR="$BUILD_DIR/cargo-target"
BIN_DIR="$BUILD_DIR/bin"

echo "==> Building doctk CLI (release)"
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p doctk-cli --manifest-path "$ROOT/Cargo.toml"

mkdir -p "$BIN_DIR"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" || "$OSTYPE" == "win32" ]]; then
  cp "$TARGET_DIR/release/doctk.exe" "$BIN_DIR/doctk.exe"
  echo "==> Built: $BIN_DIR/doctk.exe"
else
  cp "$TARGET_DIR/release/doctk" "$BIN_DIR/doctk"
  echo "==> Built: $BIN_DIR/doctk"
fi
