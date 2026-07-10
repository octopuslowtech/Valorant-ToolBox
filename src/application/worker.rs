use std::sync::mpsc::Sender;

use crate::application::installer;
use crate::domain::config::{Config, SessionData};
use crate::infrastructure::paths::{ensure_data_folder, session_data_path, valorant_config_root};
use crate::infrastructure::{display, ini, nvidia};
#[derive(Clone)]
pub enum Status {
    Idle,
    Applying,
    Done,
}

pub enum WorkerMsg {
    SetStatus(Status),
}

fn status(tx: &Sender<WorkerMsg>, s: Status) {
    let _ = tx.send(WorkerMsg::SetStatus(s));
}

pub fn run(cfg: Config, tx: Sender<WorkerMsg>) {
    status(&tx, Status::Applying);

    let root = valorant_config_root();
    ini::run_installation(&root, &cfg.x, &cfg.y);

    display::set_dpi_aware();
    let (orig_x, orig_y) = display::current_resolution();
    let orig_hz = display::current_refresh_rate();

    ensure_data_folder();
    let session_path = session_data_path();
    let target_x: u32 = cfg.x.parse().unwrap_or(0);
    let target_y: u32 = cfg.y.parse().unwrap_or(0);
    let current_is_target = orig_x == target_x as i32 && orig_y == target_y as i32;
    if !current_is_target {
        let session = SessionData {
            x: orig_x,
            y: orig_y,
            hz: orig_hz,
        };
        let _ = crate::domain::config::save_session(&session_path, &session);
    }

    if cfg.enable_nvidia_scaling {
        let (_, _msg) = nvidia::set_scaling_fullscreen();
    }

    let ids = cfg.all_instance_ids();
    if !ids.is_empty() {
        installer::disable_monitors(&ids);
    }

    let width: u32 = cfg.x.parse().unwrap_or(0);
    let height: u32 = cfg.y.parse().unwrap_or(0);
    if width > 0 && height > 0 && display::resolution_supported_at(width, height, orig_hz) {
        display::set_resolution(width, height, orig_hz);
    }

    status(&tx, Status::Done);
}
