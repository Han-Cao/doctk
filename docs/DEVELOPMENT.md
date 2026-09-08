# Development Guide

This guide covers workspace layout, dependencies, builds, and how to add new features.

## Workspace layout

```text
crates/doctk-core/     UI-agnostic core library (table, diff, text, case, pdf logic)
crates/doctk-cli/      clap-based CLI (`doctk` binary)
gui/src-tauri/         Tauri 2 backend (Rust commands wrapping core)
gui/src/               Vite + React + TypeScript frontend
docs/                  framework + per-feature implementation plans
scripts/               build scripts that keep artifacts under build/
```

## Install dependencies

### Common prerequisites (all platforms)

- **Rust** stable toolchain via [rustup](https://rustup.rs)
  - Linux/macOS: `rustup default stable`
  - Windows: use the `x86_64-pc-windows-msvc` toolchain
- **Node.js** 20 or newer (LTS recommended) and npm
- **Git**

For the GUI, install the Tauri v2 platform packages below. The CLI only needs
Rust and a C compiler; no GUI system libraries are required for `doctk-cli`.

### Linux

#### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libssl-dev \
  libdbus-1-dev
```

#### Fedora / RHEL

```bash
sudo dnf install -y \
  gcc-c++ pkgconfig \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  openssl-devel \
  dbus-devel
```

#### Arch Linux

```bash
sudo pacman -S --needed \
  base-devel \
  webkit2gtk-4.1 \
  gtk3 \
  libappindicator-gtk3 \
  librsvg \
  openssl \
  dbus \
  pkgconf
```

### macOS

```bash
xcode-select --install
```

Then make sure the Rust target for your Mac is active:

```bash
# Apple Silicon Macs
rustup target add aarch64-apple-darwin

# Intel Macs
rustup target add x86_64-apple-darwin
```

> Note: only add the target for your own architecture. Add both only if you
> intend to build a universal binary.

### Windows

Install these manually:

1. **Rust MSVC toolchain**

   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   ```

2. **Microsoft C++ Build Tools** — run the Visual Studio Installer, select the
   **“Desktop development with C++”** workload (MSVC + Windows SDK).
3. **Node.js** LTS (20 or newer).
4. **WebView2 Runtime** — usually already present on Windows 10/11. If not,
   download the Evergreen installer from Microsoft.

### Frontend packages

Once Node.js is installed, fetch the frontend dependencies:

```bash
cd gui
npm install
```

This writes `gui/node_modules/` (git-ignored) and updates `gui/package-lock.json`.

## Build the CLI

Use the prepared build scripts so all artifacts stay under the root-level
`build/` directory (nothing is written into `crates/` or `gui/`):

```bash
./scripts/build-cli.sh       # Linux/macOS -> build/bin/doctk
```

```powershell
# Windows PowerShell -> build\bin\doctk.exe
powershell -ExecutionPolicy Bypass -File .\scripts\build-cli.ps1
```

Run tests:

```bash
cargo test
```

## Build the GUI

First complete the platform setup above. Then run:

```bash
./scripts/build-gui.sh       # Linux/macOS
```

```powershell
# Windows PowerShell
powershell -ExecutionPolicy Bypass -File .\scripts\build-gui.ps1
```

The scripts redirect both frontend and Rust artifacts into:

```text
build/
├── bin/doctk                # CLI binary (copied by build-cli)
├── cargo-target/            # all Cargo artifacts
└── gui-dist/                # Vite frontend build
```

For development without the scripts:

```bash
cd gui
npm install
npm run tauri:dev
```

## Adding a new feature

Follow the checklist in [`docs/tool-framework.md`](tool-framework.md):

1. Add a core module in `crates/doctk-core/src/features/<feature_id>.rs` and register its `ToolManifest` in `registry.rs`.
2. Add a CLI command module in `crates/doctk-cli/src/commands/<feature_id>.rs` and register it in `commands/mod.rs`.
3. Add Tauri commands in `gui/src-tauri/src/features/<feature_id>.rs` and register them in `features/mod.rs`.
4. Add a frontend feature folder in `gui/src/features/<feature-id>/` and register the component in `gui/src/toolRegistry.tsx`.
