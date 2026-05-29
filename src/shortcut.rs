use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::constants::APP_NAME;
use crate::paths::{documents_dir, ensure_data_folder, permanent_icon_path};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn create_shortcut() -> bool {
    ensure_data_folder();

    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return false,
    };
    let working_dir = std::path::Path::new(&exe)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let desktop = format!(
        "{}\\Desktop",
        std::env::var("USERPROFILE").unwrap_or_default()
    );
    let shortcut_path = format!("{}\\{}.lnk", desktop, APP_NAME);

    let icon_path = permanent_icon_path();
    let icon = if icon_path.exists() {
        icon_path.to_string_lossy().to_string()
    } else {
        exe.clone()
    };

    let vbs = format!(
        "Set oShell = CreateObject(\"WScript.Shell\")\r\n\
         Set oLink = oShell.CreateShortcut(\"{shortcut}\")\r\n\
         oLink.TargetPath = \"{target}\"\r\n\
         oLink.Arguments = \"--launch\"\r\n\
         oLink.WorkingDirectory = \"{wd}\"\r\n\
         oLink.IconLocation = \"{icon}\"\r\n\
         oLink.Save\r\n",
        shortcut = shortcut_path,
        target = exe,
        wd = working_dir,
        icon = icon,
    );

    let vbs_path = documents_dir().join("_make_shortcut.vbs");
    if std::fs::write(&vbs_path, vbs).is_err() {
        return false;
    }

    let _ = Command::new("cscript")
        .args(["//Nologo", &vbs_path.to_string_lossy()])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = std::fs::remove_file(&vbs_path);

    std::path::Path::new(&shortcut_path).exists()
}

pub fn remove_shortcut() -> bool {
    let shortcut = format!(
        "{}\\Desktop\\{}.lnk",
        std::env::var("USERPROFILE").unwrap_or_default(),
        APP_NAME
    );
    let path = std::path::Path::new(&shortcut);
    if path.exists() {
        std::fs::remove_file(path).is_ok()
    } else {
        true
    }
}
