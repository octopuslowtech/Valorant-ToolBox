use std::os::windows::process::CommandExt;
use std::process::Command;

use winreg::enums::*;
use winreg::RegKey;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn create_restore_point() -> (bool, String) {
    let result = Command::new("powershell")
        .args([
            "-Command",
            "Checkpoint-Computer -Description \"Valorant-ToolBox\" -RestorePointType \"MODIFY_SETTINGS\"",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match result {
        Ok(out) if out.status.success() => (true, "Restore point created".into()),
        Ok(_) => (false, "Restore point failed (may be rate-limited)".into()),
        Err(e) => (false, e.to_string()),
    }
}

pub fn set_ultimate_power_plan() -> (bool, String) {
    let guid = "e9a42b02-d5df-448d-aa00-03f14749eb61";
    let activate = Command::new("powercfg")
        .args(["/setactive", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    if let Ok(out) = activate {
        if out.status.success() {
            return (true, "Ultimate Performance plan activated".into());
        }
    }
    let _ = Command::new("powercfg")
        .args(["/duplicatescheme", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let retry = Command::new("powercfg")
        .args(["/setactive", guid])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match retry {
        Ok(out) if out.status.success() => (true, "Ultimate Performance plan activated".into()),
        _ => (false, "Could not activate Ultimate Performance plan".into()),
    }
}

fn set_dword(hive: isize, path: &str, name: &str, value: u32) -> bool {
    let root = RegKey::predef(hive);
    if let Ok((key, _)) = root.create_subkey(path) {
        return key.set_value(name, &value).is_ok();
    }
    false
}

fn set_sz(hive: isize, path: &str, name: &str, value: &str) -> bool {
    let root = RegKey::predef(hive);
    if let Ok((key, _)) = root.create_subkey(path) {
        return key.set_value(name, &value.to_string()).is_ok();
    }
    false
}

pub fn disable_visual_effects() -> (bool, String) {
    set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects",
        "VisualFXSetting",
        2,
    );
    set_sz(HKEY_CURRENT_USER, r"Control Panel\Desktop", "MenuShowDelay", "0");
    set_dword(HKEY_CURRENT_USER, r"Control Panel\Desktop", "ForegroundLockTimeout", 0);
    set_sz(
        HKEY_CURRENT_USER,
        r"Control Panel\Desktop\WindowMetrics",
        "MinAnimate",
        "0",
    );
    (true, "Visual effects set to Best Performance".into())
}

pub fn disable_game_dvr_bar() -> (bool, String) {
    set_dword(HKEY_CURRENT_USER, r"Software\Microsoft\GameBar", "UseNexusForGameBarEnabled", 0);
    set_dword(HKEY_CURRENT_USER, r"Software\Microsoft\GameBar", "AllowAutoGameMode", 0);
    set_dword(
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\GameDVR",
        "AppCaptureEnabled",
        0,
    );
    (true, "Game Bar & DVR disabled".into())
}

pub fn disable_nagle() -> (bool, String) {
    let ok = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
        "TcpAckFrequency",
        1,
    ) && set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Services\Tcpip\Parameters",
        "TCPNoDelay",
        1,
    );
    if ok {
        (true, "Nagle's algorithm disabled".into())
    } else {
        (false, "Nagle tweak needs admin".into())
    }
}

pub fn disable_prefetch() -> (bool, String) {
    let ok = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management\PrefetchParameters",
        "EnablePrefetcher",
        0,
    ) && set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management\PrefetchParameters",
        "EnableSuperfetch",
        0,
    );
    if ok {
        (true, "Prefetch & Superfetch disabled".into())
    } else {
        (false, "Prefetch tweak needs admin".into())
    }
}

pub fn optimize_responsiveness() -> (bool, String) {
    let ok = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile",
        "SystemResponsiveness",
        0,
    );
    if ok {
        (true, "System responsiveness optimized".into())
    } else {
        (false, "Responsiveness tweak needs admin".into())
    }
}

pub fn enable_gpu_scheduling() -> (bool, String) {
    let ok = set_dword(
        HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers",
        "HwSchMode",
        2,
    );
    if ok {
        (true, "Hardware GPU scheduling enabled (restart required)".into())
    } else {
        (false, "GPU scheduling tweak needs admin".into())
    }
}

pub fn run_all<F: FnMut(String)>(mut log: F) {
    let (_, m) = create_restore_point();
    log(format!("Restore point: {}", m));
    let steps: Vec<(bool, String)> = vec![
        set_ultimate_power_plan(),
        disable_visual_effects(),
        disable_game_dvr_bar(),
        disable_nagle(),
        disable_prefetch(),
        optimize_responsiveness(),
        enable_gpu_scheduling(),
    ];
    for (ok, msg) in steps {
        log(format!("{} {}", if ok { "[OK]" } else { "[--]" }, msg));
    }
}
