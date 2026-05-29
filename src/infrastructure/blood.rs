use std::path::PathBuf;

use crate::domain::constants::BLOOD_FILES;
use crate::infrastructure::paths::backup_dir;

pub fn find_paks_dir() -> Option<PathBuf> {
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if !PathBuf::from(&drive).exists() {
            continue;
        }
        let base = PathBuf::from(&drive).join("Riot Games").join("VALORANT");
        for sub in [base.join("live"), base.clone()] {
            let paks = sub.join("ShooterGame").join("Content").join("Paks");
            if paks.exists() {
                return Some(paks);
            }
        }
    }
    None
}

pub fn emergency_cleanup() {
    let backup = backup_dir();
    if !backup.exists() {
        return;
    }
    let paks_dir = match find_paks_dir() {
        Some(p) => p,
        None => return,
    };
    if let Ok(entries) = std::fs::read_dir(&backup) {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let backup_path = entry.path();
            let game_path = paks_dir.join(&fname);
            let _ = std::fs::copy(&backup_path, &game_path);
            let _ = std::fs::remove_file(&backup_path);
        }
    }
    for fname in BLOOD_FILES {
        let game_path = paks_dir.join(fname);
        if game_path.exists() && !backup.join(fname).exists() {
            let _ = std::fs::remove_file(&game_path);
        }
    }
    let _ = std::fs::remove_dir(&backup);
}
