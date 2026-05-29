use std::path::Path;

use walkdir::WalkDir;

use crate::constants::ELITE_INI_TEMPLATE;
use crate::paths::set_read_only;

const STANDARD_RULES: &[(&str, &str)] = &[
    ("LastUserConfirmedResolutionSizeX=", "LastUserConfirmedResolutionSizeX={X}"),
    ("LastUserConfirmedResolutionSizeY=", "LastUserConfirmedResolutionSizeY={Y}"),
    ("LastUserConfirmedDesiredScreenWidth=", "LastUserConfirmedDesiredScreenWidth={X}"),
    ("LastUserConfirmedDesiredScreenHeight=", "LastUserConfirmedDesiredScreenHeight={Y}"),
    ("ResolutionSizeX=", "ResolutionSizeX={X}"),
    ("ResolutionSizeY=", "ResolutionSizeY={Y}"),
    ("DesiredScreenWidth=", "DesiredScreenWidth={X}"),
    ("DesiredScreenHeight=", "DesiredScreenHeight={Y}"),
    ("LastConfirmedFullscreenMode=", "LastConfirmedFullscreenMode=2"),
    ("PreferredFullscreenMode=", "PreferredFullscreenMode=2"),
    ("FullscreenMode=", "FullscreenMode=2"),
    ("bLastConfirmedShouldLetterbox=", "bLastConfirmedShouldLetterbox=False"),
    ("bShouldLetterbox=", "bShouldLetterbox=False"),
    ("LastConfirmedDefaultMonitorDeviceID=", "LastConfirmedDefaultMonitorDeviceID="),
    ("DefaultMonitorDeviceID=", "DefaultMonitorDeviceID="),
    ("DefaultMonitorIndex=", "DefaultMonitorIndex=0"),
];

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

fn match_rule(line: &str) -> Option<&'static str> {
    for (needle, replacement) in STANDARD_RULES {
        if line.contains(needle) {
            return Some(replacement);
        }
    }
    None
}

fn patch_standard(path: &Path, x: &str, y: &str) {
    set_read_only(path, false);
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut out = String::new();
    for line in content.lines() {
        match match_rule(line) {
            Some(repl) => {
                let replaced = repl.replace("{X}", x).replace("{Y}", y);
                out.push_str(&replaced);
            }
            None => out.push_str(line),
        }
        out.push('\n');
    }
    let _ = std::fs::write(path, out);
    set_read_only(path, true);
}

fn patch_elite(path: &Path, x: &str, y: &str) {
    set_read_only(path, false);
    let content = ELITE_INI_TEMPLATE.replace("{X}", x).replace("{Y}", y);
    let _ = std::fs::write(path, content);
    set_read_only(path, true);
}

fn apply_append_keys(content: &str) -> String {
    let mut result = content.to_string();
    for (key, val) in APPEND_KEYS {
        let prefix = format!("{}=", key);
        let kept: Vec<&str> = result
            .lines()
            .filter(|line| !line.starts_with(&prefix))
            .collect();
        let trailing_newline = result.ends_with('\n');
        result = kept.join("\n");
        if trailing_newline {
            result.push('\n');
        }

        let entry = format!("{}={}", key, val);
        if result.contains(SHOOTER_SECTION) {
            let header_with_nl = format!("{}\n", SHOOTER_SECTION);
            let injected = format!("{}\n{}\n", SHOOTER_SECTION, entry);
            result = result.replacen(&header_with_nl, &injected, 1);
        } else {
            if !result.ends_with('\n') && !result.is_empty() {
                result.push('\n');
            }
            result.push_str(&entry);
            result.push('\n');
        }
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

pub fn run_installation(root: &Path, x: &str, y: &str, perf: bool) {
    for ini_path in collect_ini_files(root) {
        set_read_only(&ini_path, false);

        if perf {
            patch_elite(&ini_path, x, y);
        } else {
            patch_standard(&ini_path, x, y);
        }

        set_read_only(&ini_path, false);
        let content = match std::fs::read_to_string(&ini_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let patched = apply_append_keys(&content);
        let _ = std::fs::write(&ini_path, patched);
        set_read_only(&ini_path, true);
    }
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
