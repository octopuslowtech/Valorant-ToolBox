use std::os::windows::process::CommandExt;
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
    if let Some(path) = find_riot_from_process() {
        return Some(path);
    }

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let candidate = PathBuf::from(&local)
            .join("Riot Games")
            .join("Riot Client")
            .join("RiotClientServices.exe");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        if !PathBuf::from(&drive).exists() {
            continue;
        }
        let bases = [
            PathBuf::from(&drive).join("Riot Games"),
            PathBuf::from(&drive).join("Program Files").join("Riot Games"),
            PathBuf::from(&drive).join("Program Files (x86)").join("Riot Games"),
        ];
        for base in bases {
            let candidate = base.join("Riot Client").join("RiotClientServices.exe");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    None
}

fn find_riot_from_process() -> Option<String> {
    let output = std::process::Command::new("wmic")
        .args(["process", "where", "name='RiotClientServices.exe'", "get", "ExecutablePath", "/value"])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("ExecutablePath=") {
            let path = path.trim();
            if !path.is_empty() && PathBuf::from(path).exists() {
                return Some(path.to_string());
            }
        }
    }
    None
}
