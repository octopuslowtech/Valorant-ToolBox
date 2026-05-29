use std::path::PathBuf;

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

pub fn get_riot_client_path() -> Option<String> {
    let keys = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Riot Games\Riot Client"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Riot Games\Riot Client"),
        (HKEY_CURRENT_USER, r"SOFTWARE\Riot Games\Riot Client"),
        (HKEY_CURRENT_USER, r"SOFTWARE\WOW6432Node\Riot Games\Riot Client"),
    ];

    for (hive, path) in keys {
        let root = RegKey::predef(hive);
        if let Ok(key) = root.open_subkey(path) {
            if let Ok(folder) = key.get_value::<String, _>("InstallFolder") {
                let candidate = PathBuf::from(folder).join("RiotClientServices.exe");
                if candidate.exists() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

pub fn scan_drives_for_riot() -> Option<String> {
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if !PathBuf::from(&drive).exists() {
            continue;
        }
        let candidate = PathBuf::from(&drive)
            .join("Riot Games")
            .join("Riot Client")
            .join("RiotClientServices.exe");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}
