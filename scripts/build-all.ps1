# Build CLI and GUI in sequence.
$ErrorActionPreference = "Stop"

$ROOT = Resolve-Path (Join-Path $PSScriptRoot "..")
& (Join-Path $ROOT "scripts\build-cli.ps1")
& (Join-Path $ROOT "scripts\build-gui.ps1")
