use std::time::{Duration, Instant};

use crate::config::{self, SessionData};
use crate::display;
use crate::ini;
use crate::logger::Logger;
use crate::paths::{
    config_path, ensure_data_folder, log_path, session_data_path, valorant_config_root,
};
use crate::process::is_process_running;
use crate::riot::{get_riot_client_path, scan_drives_for_riot};
use crate::{admin, dialog};

const STARTUP_TIMEOUT: u64 = 300;
const POLL_INTERVAL: u64 = 3;
const WATCH_DURATION: u64 = 90;
const NO_CHANGE_EXIT: u64 = 30;

fn list_subfolders(root: &std::path::Path) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name != "CrashReportClient" {
                    set.insert(name);
                }
            }
        }
    }
    set
}

fn valorant_running() -> bool {
    is_process_running("VALORANT-Win64-Shipping.exe") || is_process_running("VALORANT.exe")
}

pub fn launch_toolbox() {
    let cfg_path = config_path();
    if !cfg_path.exists() {
        return;
    }

    ensure_data_folder();
    let log = Logger::create(log_path());

    let cfg = match config::load_config(&cfg_path) {
        Some(c) => c,
        None => return,
    };
    log.log(&format!(
        "Config: stretch={}x{} perf={}",
        cfg.x, cfg.y, cfg.perf
    ));

    let root = valorant_config_root();
    if !root.exists() {
        log.log("WARNING: Valorant config root not found");
    }

    log.log("Patching INI files before launch...");
    ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
    log.log("Pre-launch patch complete.");

    display::set_dpi_aware();
    let (orig_x, orig_y) = display::current_resolution();
    log.log(&format!("Native resolution captured: {}x{}", orig_x, orig_y));

    let orig_hz = display::current_refresh_rate();
    log.log(&format!("Refresh rate captured: {}hz", orig_hz));

    let _ = config::save_session(
        &session_data_path(),
        &SessionData {
            x: orig_x,
            y: orig_y,
            hz: orig_hz,
        },
    );

    let width: u32 = cfg.x.parse().unwrap_or(0);
    let height: u32 = cfg.y.parse().unwrap_or(0);
    log.log(&format!("Applying stretch: {}x{} @ {}hz", width, height, orig_hz));
    let applied = display::set_resolution(width, height, orig_hz);
    log.log(&format!("set_resolution result: {}", applied));

    let (check_x, check_y) = display::current_resolution();
    log.log(&format!(
        "Resolution after stretch apply: {}x{} (expected {}x{})",
        check_x, check_y, cfg.x, cfg.y
    ));

    let mut riot_path = get_riot_client_path();
    log.log(&format!(
        "Registry Riot Client path: {}",
        riot_path.clone().unwrap_or_else(|| "NOT FOUND in registry".into())
    ));
    if riot_path.is_none() {
        riot_path = scan_drives_for_riot();
    }
    log.log(&format!(
        "Final Riot Client path: {}",
        riot_path.clone().unwrap_or_else(|| "NOT FOUND".into())
    ));

    let riot_path = match riot_path {
        Some(p) => p,
        None => {
            dialog::error("Error", "Riot Client not found. Restoring resolution.");
            restore_and_exit(orig_x, orig_y);
            return;
        }
    };

    log.log("Launching Riot Client...");
    let riot_dir = std::path::Path::new(&riot_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    admin::shell_open(
        &riot_path,
        "--launch-product=valorant --launch-patchline=live",
        &riot_dir,
    );
    log.log("Riot Client launched.");

    let folders_before = list_subfolders(&root);
    log.log(&format!(
        "Config folders before launch: {} folders found",
        folders_before.len()
    ));

    log.log("Phase A - waiting for VALORANT-Win64-Shipping.exe...");
    let start_wait = Instant::now();
    loop {
        let elapsed = start_wait.elapsed().as_secs();
        if is_process_running("VALORANT-Win64-Shipping.exe") {
            log.log(&format!("Phase A - VALORANT detected after {}s.", elapsed));
            let folders_now = list_subfolders(&root);
            let new_folders: Vec<_> = folders_now.difference(&folders_before).collect();
            if !new_folders.is_empty() {
                log.log(&format!(
                    "New folders detected: {:?} - patching immediately",
                    new_folders
                ));
                ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
            }
            break;
        }
        if !is_process_running("RiotClientServices.exe") && elapsed > 180 {
            log.log("Phase A - Riot Client closed before Valorant appeared. Restoring.");
            restore_and_exit(orig_x, orig_y);
            return;
        }
        if elapsed > STARTUP_TIMEOUT {
            log.log("Phase A - Timed out. Restoring.");
            restore_and_exit(orig_x, orig_y);
            return;
        }
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL));
    }

    log.log("Watching INI files for Valorant cloud sync writes (90s max)...");
    let watch_start = Instant::now();
    let mut last_mtimes = ini::ini_mtimes(&root);
    let mut patch_fired = false;
    let mut last_change_time = Instant::now();

    while watch_start.elapsed().as_secs() < WATCH_DURATION {
        std::thread::sleep(Duration::from_secs(3));

        if !valorant_running() {
            log.log("Valorant closed during watch window - patching and exiting");
            ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
            patch_fired = true;
            break;
        }

        let current_mtimes = ini::ini_mtimes(&root);
        let changed: Vec<_> = current_mtimes
            .iter()
            .filter(|(p, t)| last_mtimes.get(*p) != Some(*t))
            .map(|(p, _)| p.clone())
            .collect();

        if !changed.is_empty() {
            log.log(&format!("INI change detected: {:?} - patching now", changed));
            ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
            last_mtimes = ini::ini_mtimes(&root);
            last_change_time = Instant::now();
            patch_fired = true;
        } else if patch_fired && last_change_time.elapsed().as_secs() > NO_CHANGE_EXIT {
            log.log("No further changes for 30s after patch - exiting watch early");
            break;
        }
    }

    if !patch_fired {
        log.log("No INI changes detected - patching as safety net");
        ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
    }

    log.log("INI watch complete.");

    if !valorant_running() {
        log.log("Valorant already closed - final patch then restoring.");
        ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
        restore_and_exit(orig_x, orig_y);
        return;
    }

    log.log("Phase B - monitoring for Valorant close...");
    loop {
        if !valorant_running() {
            log.log("Phase B - confirmed closed. Restoring.");
            break;
        }
        std::thread::sleep(Duration::from_secs(POLL_INTERVAL));
    }

    log.log("Phase C - final INI patch after Valorant closed...");
    ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
    log.log("Phase C - restoring resolution and exiting.");
    restore_and_exit(orig_x, orig_y);
}

pub fn restore_and_exit(fallback_x: i32, fallback_y: i32) {
    let (mut res_x, mut res_y, mut res_hz) = (fallback_x, fallback_y, 60u32);
    if let Some(data) = config::load_session(&session_data_path()) {
        res_x = data.x;
        res_y = data.y;
        res_hz = data.hz;
    }

    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(8));
        std::process::exit(0);
    });

    display::set_resolution(res_x as u32, res_y as u32, res_hz);

    let vbs_path = crate::paths::documents_dir().join("_make_shortcut.vbs");
    let _ = std::fs::remove_file(&vbs_path);

    std::process::exit(0);
}
