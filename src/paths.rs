use std::path::PathBuf;

use crate::constants::{APP_NAME, CONFIG_FILE};

pub fn documents_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    PathBuf::from(home).join("Documents").join(APP_NAME)
}

pub fn session_data_path() -> PathBuf {
    documents_dir().join("native_res.json")
}

pub fn permanent_icon_path() -> PathBuf {
    documents_dir().join("redyellow.ico")
}

pub fn config_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    PathBuf::from(appdata).join(CONFIG_FILE)
}

pub fn log_path() -> PathBuf {
    documents_dir().join("debug.log")
}

pub fn valorant_config_root() -> PathBuf {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    PathBuf::from(local)
        .join("VALORANT")
        .join("Saved")
        .join("Config")
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resource_path(name: &str) -> PathBuf {
    exe_dir().join(name)
}

pub fn ensure_data_folder() {
    let dir = documents_dir();
    let _ = std::fs::create_dir_all(&dir);
    let bundled = resource_path("redyellow.ico");
    let target = permanent_icon_path();
    if bundled.exists() && !target.exists() {
        let _ = std::fs::copy(&bundled, &target);
    }
}

pub fn set_read_only(path: &std::path::Path, read_only: bool) {
    if !path.exists() {
        return;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_readonly(read_only);
        let _ = std::fs::set_permissions(path, perms);
    }
}

pub fn backup_dir() -> PathBuf {
    documents_dir().join(".originals_backup")
}

