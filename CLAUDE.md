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

Windows-only Rust GUI app (eframe/egui) that applies stretched resolutions for Valorant and patches game INI files.

### Two execution modes

1. **GUI mode** (default) — `app.rs` runs the egui window with system tray. User picks resolution, clicks Apply. A background worker thread (`worker.rs`) patches INIs, sets NVIDIA scaling, disables extra monitors, and changes display resolution.

2. **Launcher mode** (`--launch`) — `launcher.rs` runs headless from a desktop shortcut. Captures native resolution, applies stretch, launches Riot Client, watches for Valorant process and INI cloud-sync writes, then restores resolution when Valorant closes. Three phases: wait for game start → watch INI changes (90s) → wait for game exit → restore.

### Key modules

- `config.rs` — `Config` and `SessionData` structs, JSON persistence to `%APPDATA%\ValorantToolBoxConfig.json`
- `display.rs` — Win32 `ChangeDisplaySettingsExW` wrappers for resolution get/set/validate
- `ini.rs` — Patches `GameUserSettings.ini` files under `%LOCALAPPDATA%\VALORANT\Saved\Config\`. Two modes: standard (line-by-line replacement) and elite/perf (full template from `constants.rs`)
- `blood.rs` — Copies/removes mature content `.pak/.ucas/.utoc/.sig` files to/from Valorant's Paks directory. Emergency cleanup on exit restores originals from backup.
- `nvidia.rs` — Recursively sets `Scaling=3` (fullscreen) in registry under `GraphicsDrivers\Configuration`
- `monitors.rs` — Enumerates display devices from registry; `installer.rs` disables/enables via `pnputil`
- `launcher.rs` — The `--launch` shortcut flow with process polling and INI mtime watching
- `paths.rs` — All filesystem paths (Documents, AppData, session data, backups)
- `riot.rs` — Locates Riot Client via registry or drive scan

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
