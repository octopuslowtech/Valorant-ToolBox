use std::sync::mpsc::Sender;

use crate::config::{Config, SessionData};
use crate::paths::{ensure_data_folder, session_data_path, valorant_config_root};
use crate::{display, ini, installer, nvidia};
#[derive(Clone)]
pub enum Status {
    Idle,
    Applying,
    Done,
    Error(String),
}

pub enum WorkerMsg {
    Log(String),
    SetStatus(Status),
}

fn log(tx: &Sender<WorkerMsg>, msg: impl Into<String>) {
    let _ = tx.send(WorkerMsg::Log(msg.into()));
}

fn status(tx: &Sender<WorkerMsg>, s: Status) {
    let _ = tx.send(WorkerMsg::SetStatus(s));
}

pub fn run(cfg: Config, tx: Sender<WorkerMsg>) {
    status(&tx, Status::Applying);

    let root = valorant_config_root();
    log(&tx, "Patching INI files...");
    ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);

    display::set_dpi_aware();
    let (orig_x, orig_y) = display::current_resolution();
    let orig_hz = display::current_refresh_rate();
    log(&tx, format!("Native resolution: {}x{} @ {}hz", orig_x, orig_y, orig_hz));

    ensure_data_folder();
    let session_path = session_data_path();
    if !session_path.exists() {
        let session = SessionData { x: orig_x, y: orig_y, hz: orig_hz };
        let _ = crate::config::save_session(&session_path, &session);
    }

    if cfg.enable_nvidia_scaling {
        let (_, msg) = nvidia::set_scaling_fullscreen();
        log(&tx, msg);
    }

    let ids = cfg.all_instance_ids();
    if !ids.is_empty() {
        log(&tx, format!("Disabling {} monitor(s)...", ids.len()));
        installer::disable_monitors(&ids);
        log(&tx, "Monitors disabled.".to_string());
    }

    let width: u32 = cfg.x.parse().unwrap_or(0);
    let height: u32 = cfg.y.parse().unwrap_or(0);
    let applied = display::set_resolution(width, height, orig_hz);
    log(&tx, format!("Applied stretch {}x{}: {}", width, height, applied));

    log(&tx, "Done.");
    status(&tx, Status::Done);
}
