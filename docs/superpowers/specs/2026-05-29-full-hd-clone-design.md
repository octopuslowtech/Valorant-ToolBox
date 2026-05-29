# Valorant-ToolBox — Full HiddenDisplay Clone Design

Date: 2026-05-29

## Goal

Port all HiddenDisplay (Python) features into the existing Rust egui app while keeping the
current visual style. Transform the app from a config-only installer into a resident GUI
launcher with live Play, mod injection, FPS tweaks, and tray support.

## Decisions

- App model: resident GUI (Play button + live log + tray), like HD.
- Feature scope: blood mod + VNG logo removal, FPS registry tweaks, auto NVIDIA scaling,
  NVIDIA LOD profile (NPI).
- Close behavior: tray + watcher. Closing the window minimizes to tray; the cleanup
  watcher keeps running so mods and resolution restore on game exit. Only "Quit" exits.
- Riot launch: auto-Play via Riot local API (lockfile + reqwest POST).
- UI layout: keep Overview + Advanced tabs; add Mods + Performance tabs. PLAY button and
  live log always visible at the bottom of every tab.
- Language: bilingual en/vi, switchable in Advanced.
- Tech: reqwest (blocking, accept invalid certs) for the Riot API; assets shipped as an
  external portable bundle next to the exe (`blood/`, `npi/`), not embedded.

## Architecture

- `main.rs`: admin elevation on startup, single-instance named mutex
  (`Global\ValorantToolBox`), keep legacy args (`--install-direct`, `--uninstall-direct`,
  `--launch`), default to resident GUI.
- GUI thread (egui) <-> worker thread via `Arc<Mutex<AppState>>` + `mpsc` channel. Worker
  sends `LogMsg` / `StatusMsg`; UI polls in `update()` and calls `ctx.request_repaint()`.
- Worker never blocks the UI.

## Module map

New modules:
- `app.rs`: resident GUI — 4 tabs + Play + log + tray (replaces `gui.rs`).
- `riot_api.rs`: reqwest read lockfile, ping `/riotclient/region-locale`, POST
  `product-launcher`.
- `blood.rs`: race-inject blood pak + remove VNG, backup to `.originals_backup`, cleanup
  watcher, emergency cleanup.
- `fps.rs`: FPS registry tweaks, Ultimate power plan, timer resolution, restore point.
- `nvidia.rs`: auto scaling HKLM `GraphicsDrivers\Configuration\...\Scaling=3` + NPI LOD.
- `lang.rs`: `LANG` en/vi, `t(key)`.
- `worker.rs`: Play pipeline state machine.

Modified:
- `config.rs`: + enable_blood, enable_vng_remove, enable_nvidia_scaling, lod_preset,
  language, minimize_to_tray.
- `constants.rs`: + blood/VNG filename lists, NPI setting IDs.
- `paths.rs`: + blood_dir(), npi_exe(), backup_dir(), lockfile_path().
- `process.rs`: + GPU category (nvidia / hybrid / non-nvidia).
- `launcher.rs`: keep for `--launch` headless; share pipeline with worker.

Unchanged (reused): display, ini, monitors, riot, installer, startup, shortcut, admin,
dialog, logger.

## Play pipeline (worker thread)

1. Pre-apply stretch (ini + display::set_resolution), save native res/hz to session.
2. Find Riot Client (registry -> drive scan).
3. Clean Riot state (kill RiotClientServices + delete stale lockfile).
4. Start Riot fresh (`cmd /c start` to de-elevate), wait lockfile (<=15s).
5. Wait API ready (ping region-locale, <=20s).
6. Trigger Play (POST product-launcher, retry 3x).
7. Poll game process (50ms, <=5min).
8. RACE INJECT: blood copy + VNG remove, backup originals.
9. Re-patch INI if new config folders appear (cloud-sync defense, existing phase A/B/C).
10. Spawn cleanup watcher: wait game exit -> restore mods + resolution.

`StatusMsg` enum: Launching / Detected / Injected{ms} / Watching / Restored / Error{msg}
drives button color + text.

## Dependencies to add

- `reqwest` (blocking, danger_accept_invalid_certs).
- `tray-icon` for system tray.

## Assets (portable bundle next to exe)

- `blood/MatureData-WindowsClient.{pak,sig,ucas,utoc}`
- `npi/nvidiaProfileInspector.exe`

## Safety notes

- Tray keeps watcher alive; Quit triggers emergency cleanup.
- FPS/NVIDIA HKLM tweaks + pnputil require admin (elevated at startup).
- Restore point created before registry tweaks.
- Blood/VNG ban risk: low; originals restored on game exit.
