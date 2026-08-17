#!/usr/bin/env bash
# Build CLI and GUI in sequence.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
"$ROOT/scripts/build-cli.sh"
"$ROOT/scripts/build-gui.sh"
