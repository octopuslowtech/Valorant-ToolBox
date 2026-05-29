<p align="center">
  <img src="redyellow.ico" alt="Valorant-ToolBox" width="96" height="96">
</p>

<h1 align="center">Valorant-ToolBox</h1>

<p align="center">
  <strong>Stretched resolution launcher & system optimizer for Valorant on Windows</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-blue?logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/language-Rust-orange?logo=rust" alt="Language">
  <img src="https://img.shields.io/badge/GUI-egui%2Feframe-green" alt="GUI">
  <img src="https://img.shields.io/badge/version-2.0.0-brightgreen" alt="Version">
  <img src="https://img.shields.io/badge/license-All%20Rights%20Reserved-red" alt="License">
</p>

---

## Overview

Valorant-ToolBox is a native Windows application that automates stretched resolution setup, game INI patching, NVIDIA GPU configuration, and system-level performance optimizations for competitive Valorant gameplay. Built in Rust with a clean egui/eframe GUI and system tray integration.

## Features

### Display & Resolution
- Stretched resolution presets (1440x1080, 1280x960, 1024x768) and custom input
- Automatic display mode switching via Win32 `ChangeDisplaySettingsExW`
- Multi-monitor management — disable secondary displays during gameplay
- NVIDIA fullscreen scaling override (registry-based GPU scaling)

### Game Configuration
- INI patching for `GameUserSettings.ini` with read-only lock to prevent cloud sync overwrite
- Valorant graphics presets (Low/Medium/High/Custom)
- Blood/mature content toggle via pak file management
- FPS and performance registry tweaks

### NVIDIA Integration
- GPU scaling mode (Scaling=3) via registry
- Digital vibrance control through NvAPI

### System Optimization
- High Performance / Ultimate Performance power plan activation
- Background services disabling (telemetry, indexing, unused services)
- Network stack optimization (TCP tuning, RSS, auto-tuning)
- Process priority elevation for Valorant and Riot Client
- Background apps and startup delay removal
- Hardware-accelerated GPU scheduling registry tweak

### Automation
- **Launcher mode** — one-click desktop shortcut that applies settings, launches Riot Client, monitors Valorant process, and auto-reverts on game exit
- **Run on startup** via Windows Task Scheduler
- **Silent install** with CLI arguments for unattended setup
- System tray with minimize-to-tray support

### Localization
- English and Vietnamese UI

## Requirements

| Requirement | Details |
|-------------|---------|
| OS | Windows 10 / 11 (64-bit) |
| Privileges | Administrator (UAC elevation) |
| GPU | NVIDIA (required for scaling and vibrance features) |
| Runtime | None — single static binary |

## Installation

Download the latest release binary or build from source:

```bash
cargo build --release
```

The output binary is located at `target/release/Valorant-ToolBox.exe`.

## Usage

### GUI Mode

```bash
Valorant-ToolBox.exe
```

Select resolution, toggle features, click **Apply**. Click **Revert** to restore original settings. Minimize to system tray.

### Launcher Mode

```bash
Valorant-ToolBox.exe --launch
```

Headless execution designed for desktop shortcuts:
1. Captures native resolution
2. Applies stretched resolution + all configured tweaks
3. Launches Riot Client
4. Watches for Valorant process and INI cloud-sync writes
5. Restores all settings when Valorant closes

### Silent Install

```bash
Valorant-ToolBox.exe --install-direct --res-x=1440 --res-y=1080 --perf=1 --monitors=Monitor1,Monitor2
```

### Uninstall

```bash
Valorant-ToolBox.exe --uninstall-direct
```

## Build

```bash
cargo build                # Debug build
cargo build --release      # Release build (opt-level=z, LTO, strip, panic=abort)
cargo clippy               # Lint
cargo fmt                  # Format
```

Release profile produces a minimal binary with:
- `opt-level = "z"` (size optimization)
- LTO enabled
- Single codegen unit
- Symbols stripped
- Panic = abort

## Architecture

Clean architecture with four layers and strict dependency direction:

```
presentation → application → infrastructure → domain
```

```
src/
├── main.rs                  Entry point, CLI routing, single-instance mutex
├── admin.rs                 UAC elevation
├── domain/                  Core types (no OS dependencies)
│   ├── config.rs            Config struct, SessionData, JSON persistence
│   └── constants.rs         App name, INI templates, resolution presets
├── infrastructure/          OS/hardware interaction
│   ├── blood.rs             Mature content pak file management
│   ├── display.rs           Win32 display mode switching
│   ├── fps.rs               Performance registry tweaks
│   ├── graphics.rs          Valorant graphics presets
│   ├── ini.rs               GameUserSettings.ini patching
│   ├── monitors.rs          Display device enumeration
│   ├── nvidia.rs            NVIDIA scaling registry
│   ├── optimize.rs          System optimization (services, network, registry, priority)
│   ├── paths.rs             Filesystem paths (AppData, Documents, backups)
│   ├── process.rs           Process detection, pnputil device control
│   ├── riot.rs              Riot Client path discovery
│   └── vibrance.rs          NVIDIA digital vibrance (NvAPI)
├── application/             Use cases and orchestration
│   ├── installer.rs         Install/uninstall, shortcut management
│   ├── launcher.rs          --launch headless mode with process watching
│   ├── startup.rs           Task Scheduler startup registration
│   └── worker.rs            Background worker thread (Apply/Revert)
└── presentation/            UI layer
    ├── app.rs               eframe/egui window + system tray
    ├── dialog.rs            Win32 MessageBox wrappers
    ├── lang.rs              i18n translation strings
    ├── logger.rs            In-app log panel
    └── shortcut.rs          Desktop/Start Menu shortcut creation
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [eframe](https://crates.io/crates/eframe) / [egui](https://crates.io/crates/egui) | Immediate-mode GUI framework |
| [windows](https://crates.io/crates/windows) | Win32 API bindings |
| [winreg](https://crates.io/crates/winreg) | Windows Registry access |
| [serde](https://crates.io/crates/serde) / [serde_json](https://crates.io/crates/serde_json) | Configuration serialization |
| [tray-icon](https://crates.io/crates/tray-icon) | System tray integration |
| [walkdir](https://crates.io/crates/walkdir) | Recursive directory traversal |
| [image](https://crates.io/crates/image) | Icon loading (ICO format) |
| [reqwest](https://crates.io/crates/reqwest) | HTTP client (update checks) |

## How It Works

```
User clicks Apply
  → build_config() from UI state
  → save config to JSON
  → spawn background worker thread
      → patch GameUserSettings.ini
      → set INI read-only (prevent cloud sync)
      → save session (native resolution backup)
      → set NVIDIA GPU scaling
      → disable secondary monitors
      → change display resolution

User clicks Revert
  → load saved session
  → restore native resolution
  → re-enable monitors
  → remove INI read-only lock
```

## Security Notes

- Requires administrator privileges for registry modifications, service control, and display settings
- Single-instance enforcement via Windows named mutex
- No network communication except optional update checks (HTTPS via rustls)
- No telemetry or data collection

## License

All rights reserved.

---

<p align="center">
  Built with Rust for competitive Valorant players.
</p>
