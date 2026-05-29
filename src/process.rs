use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const DETACHED_PROCESS: u32 = 0x00000008;

pub fn is_process_running(name: &str) -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", name), "/NH", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            text.contains(&name.to_lowercase())
        }
        Err(_) => false,
    }
}

pub fn has_nvidia_gpu() -> bool {
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .to_lowercase()
            .contains("nvidia"),
        Err(_) => false,
    }
}

pub fn pnputil_disable(instance_id: &str) {
    let _ = Command::new("pnputil")
        .args(["/disable-device", instance_id])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .output();
}

pub fn pnputil_enable(instance_id: &str) {
    let _ = Command::new("pnputil")
        .args(["/enable-device", instance_id])
        .creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)
        .output();
}


pub fn gpu_names() -> Vec<String> {
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && l.to_lowercase() != "name")
            .collect(),
        Err(_) => Vec::new(),
    }
}

