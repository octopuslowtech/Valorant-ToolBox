use std::path::Path;

use walkdir::WalkDir;

use crate::domain::constants::ELITE_INI_TEMPLATE;
use crate::infrastructure::paths::set_read_only;

const APPEND_KEYS: &[(&str, &str)] = &[
    ("bShouldLetterbox", "False"),
    ("bLastConfirmedShouldLetterbox", "False"),
    ("FullscreenMode", "2"),
    ("PreferredFullscreenMode", "2"),
    ("LastConfirmedFullscreenMode", "2"),
    ("LastConfirmedDefaultMonitorIndex", "0"),
    ("DefaultMonitorIndex", "0"),
    ("DefaultMonitorDeviceID", ""),
    ("LastConfirmedDefaultMonitorDeviceID", ""),
];

const SHOOTER_SECTION: &str = "[/Script/ShooterGame.ShooterGameUserSettings]";

fn apply_append_keys(content: &str) -> String {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let trailing_newline = content.ends_with('\n');
    let prefixes: Vec<String> = APPEND_KEYS
        .iter()
        .map(|(key, _)| format!("{}=", key))
        .collect();
    let mut lines: Vec<String> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !prefixes.iter().any(|prefix| trimmed.starts_with(prefix))
        })
        .map(|line| line.to_string())
        .collect();
    let entries: Vec<String> = APPEND_KEYS
        .iter()
        .map(|(key, val)| format!("{}={}", key, val))
        .collect();

    if let Some(pos) = lines.iter().position(|line| line.trim() == SHOOTER_SECTION) {
        for (idx, entry) in entries.into_iter().enumerate() {
            lines.insert(pos + 1 + idx, entry);
        }
    } else {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(SHOOTER_SECTION.to_string());
        lines.extend(entries);
    }

    let mut result = lines.join(newline);
    if trailing_newline || !result.is_empty() {
        result.push_str(newline);
    }
    result
}

pub fn collect_ini_files(root: &Path) -> Vec<std::path::PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "GameUserSettings.ini")
        .map(|e| e.path().to_path_buf())
        .filter(|p| !p.to_string_lossy().contains("CrashReportClient"))
        .collect()
}

pub fn run_installation(root: &Path, x: &str, y: &str) -> usize {
    let mut patched_count = 0;
    for ini_path in collect_ini_files(root) {
        set_read_only(&ini_path, false);
        let content = ELITE_INI_TEMPLATE.replace("{X}", x).replace("{Y}", y);
        let patched = apply_append_keys(&content);
        if std::fs::write(&ini_path, patched).is_ok() {
            patched_count += 1;
        }
        set_read_only(&ini_path, true);
    }
    patched_count
}

pub fn unlock_all_inis(root: &Path) -> usize {
    let mut count = 0;
    for ini_path in collect_ini_files(root) {
        set_read_only(&ini_path, false);
        count += 1;
    }
    count
}

pub fn ini_mtimes(root: &Path) -> std::collections::HashMap<String, std::time::SystemTime> {
    let mut map = std::collections::HashMap::new();
    for ini_path in collect_ini_files(root) {
        if let Ok(meta) = std::fs::metadata(&ini_path) {
            if let Ok(mtime) = meta.modified() {
                map.insert(ini_path.to_string_lossy().to_string(), mtime);
            }
        }
    }
    map
}
