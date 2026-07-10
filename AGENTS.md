# AGENTS.md

This file gives coding agents the current repo context and constraints.

## Build Commands

```bash
cargo build
cargo build --release
cargo run
cargo clippy
cargo fmt
```

No test suite exists. App is Windows-only and requires admin privileges for real execution.

## Project Summary

`Valorant-ToolBox` is a Windows Rust GUI app using `eframe/egui`. It patches Valorant `GameUserSettings.ini`, applies stretched display resolutions, can set NVIDIA scaling, can adjust digital vibrance, can disable selected monitors through `pnputil`, and can launch Valorant through Riot Client.

## Architecture

Clean architecture with four layers:

```text
presentation -> application -> infrastructure -> domain
```

### Current Source Tree

```text
src/
├── main.rs              # Entry point, CLI routing, admin elevation, single-instance mutex
├── admin.rs             # UAC/admin helpers and ShellExecute wrappers
├── domain/              # Core data and constants
│   ├── config.rs        # Config, MonitorSelection, SessionData, JSON persistence
│   └── constants.rs     # App name, config filename, Valorant INI template
├── infrastructure/      # OS and hardware interaction
│   ├── display.rs       # Win32 display mode, custom resolution, DPI awareness
│   ├── ini.rs           # Valorant GameUserSettings.ini patching and locking
│   ├── monitors.rs      # Display and monitor enumeration
│   ├── nvidia.rs        # NVIDIA fullscreen scaling registry changes
│   ├── paths.rs         # App data, Documents, Valorant config paths
│   ├── process.rs       # Process checks, GPU detection, pnputil wrappers
│   ├── riot.rs          # Riot Client path discovery
│   └── vibrance.rs      # NVIDIA digital vibrance state and worker
├── application/         # Use cases and orchestration
│   ├── installer.rs     # Install/uninstall, config, INI patch, shortcut flow
│   ├── launcher.rs      # --launch headless launcher and restore flow
│   ├── startup.rs       # Windows startup registration
│   └── worker.rs        # Background Apply/Revert worker
└── presentation/        # UI and shell integration
    ├── app.rs           # egui app, tabs, tray, apply/revert actions
    ├── dialog.rs        # Win32 MessageBox helpers
    ├── lang.rs          # UI strings and language selection
    ├── logger.rs        # File logger
    └── shortcut.rs      # Desktop shortcut creation/removal
```

## Execution Modes

### GUI mode

Default path in `presentation/app.rs`:

```text
main -> presentation::app::run -> ToolboxApp
```

User chooses preset/custom resolution, NVIDIA scaling, tray/startup/vibrance options, then starts worker:

```text
build_config -> save_config -> worker::run
```

Worker patches INIs, stores native resolution/session data, applies NVIDIA scaling if enabled, disables configured monitors, then changes display resolution.

### Launcher mode

`--launch` path in `application/launcher.rs`:

```text
main -> launcher::launch_toolbox
```

Launcher loads saved config, patches INIs, captures native resolution and refresh rate, applies stretch, finds Riot Client, launches Valorant, watches Valorant process and INI cloud-sync writes, patches again when needed, then restores native resolution when Valorant exits.

### Direct install/uninstall

```text
--install-direct --res-x=<width> --res-y=<height> --monitors=<encoded selections>
--uninstall-direct
```

Direct install validates resolution, saves config, patches INIs, registers custom resolution, warns about NVIDIA Control Panel scaling override, and creates desktop shortcut. Direct uninstall re-enables configured monitors, unlocks INIs, removes config, and removes shortcut.

## Data Files

Runtime data lives under the paths resolved by `infrastructure/paths.rs`, including:

- `ValorantToolBoxConfig.json`
- session data used for resolution restore
- launcher log file
- Valorant config root under user AppData

## Important Constraints

- Keep dependencies minimal. Do not add crates without clear need.
- App must keep admin elevation path intact.
- Single-instance mutex applies to GUI mode only.
- `set_read_only`/unlock behavior in `ini.rs` protects patched Valorant INIs from cloud sync overwrites.
- Restore flow must preserve original resolution and refresh rate when possible.
- Do not assume tests exist. Use `cargo fmt --check`, `cargo clippy`, and `cargo build` for verification.
- Do not modify `target/` or `target_verify/` artifacts.
- Keep generated code and technical artifacts in English.
