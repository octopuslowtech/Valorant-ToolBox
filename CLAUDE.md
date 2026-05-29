# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                # debug build
cargo build --release      # release build (opt-level=z, LTO, strip)
cargo run                  # run (requires admin elevation on Windows)
cargo clippy               # lint
cargo fmt                  # format
```

No test suite exists. The app requires Windows and admin privileges to run.

## Architecture

Windows-only Rust GUI app (eframe/egui) that applies stretched resolutions for Valorant and patches game INI files. Clean architecture with four layers.

### Layer Structure

```
src/
├── main.rs              # Entry point, CLI routing, single-instance mutex
├── admin.rs             # UAC elevation
├── domain/              # Core types (no OS dependencies)
│   ├── config.rs        # Config, SessionData, JSON persistence
│   └── constants.rs     # APP_NAME, INI templates, resolution presets
├── infrastructure/      # OS/hardware interaction
│   ├── blood.rs         # Mature content pak management
│   ├── display.rs       # Win32 ChangeDisplaySettingsExW
│   ├── fps.rs           # Performance registry tweaks
│   ├── graphics.rs      # Valorant graphics presets
│   ├── ini.rs           # GameUserSettings.ini patching
│   ├── monitors.rs      # Display device enumeration
│   ├── nvidia.rs        # NVIDIA scaling registry
│   ├── optimize.rs      # System optimization (services, network, registry, priority)
│   ├── paths.rs         # Filesystem paths (AppData, Documents)
│   ├── process.rs       # Process detection, pnputil
│   ├── riot.rs          # Riot Client path discovery
│   └── vibrance.rs      # NVIDIA digital vibrance (NvAPI)
├── application/         # Use cases, orchestration
│   ├── installer.rs     # Install/uninstall, shortcut management
│   ├── launcher.rs      # --launch headless mode
│   ├── startup.rs       # Task Scheduler registration
│   └── worker.rs        # Background Apply/Revert thread
└── presentation/        # UI
    ├── app.rs           # egui window + system tray
    ├── dialog.rs        # Win32 MessageBox
    ├── lang.rs          # i18n strings
    ├── logger.rs        # In-app log panel
    └── shortcut.rs      # Desktop shortcut creation
```

### Layer Dependencies

```
presentation → application → infrastructure → domain
```

### Two execution modes

1. **GUI mode** (default) — `presentation/app.rs` runs the egui window with system tray. User picks resolution, clicks Apply. A background worker thread (`application/worker.rs`) patches INIs, sets NVIDIA scaling, disables extra monitors, and changes display resolution.

2. **Launcher mode** (`--launch`) — `application/launcher.rs` runs headless from a desktop shortcut. Captures native resolution, applies stretch, launches Riot Client, watches for Valorant process and INI cloud-sync writes, then restores resolution when Valorant closes.

### Data flow

```
User clicks Apply → build_config() → save_config → spawn worker thread
Worker: patch INIs → save session (native res) → set NVIDIA scaling → disable monitors → set_resolution
User clicks Revert → load session → set_resolution(original) → enable monitors
```

### Important constraints

- App requires admin (UAC elevation in `main.rs`). Single-instance enforced via named mutex.
- `set_read_only` is used to lock INI files after patching (prevents Valorant cloud sync from overwriting).
- `blood/` directory contains binary game asset files bundled into the release — do not modify.
- `build.rs` embeds `redyellow.ico` and `app.manifest` via winresource.
- The `docs/` directory contains reference implementations (Python `HiddenDisplay`, C# `vibranceGUI`) — not part of the build.
