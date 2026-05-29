use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn is_admin() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn elevate(args: &str) -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => return false,
    };
    let exe_w = to_wide(&exe);
    let args_w = to_wide(args);
    unsafe {
        let result = ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR::from_raw(exe_w.as_ptr()),
            PCWSTR::from_raw(args_w.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        result.0 as isize > 32
    }
}

pub fn shell_open(target: &str, args: &str, working_dir: &str) {
    let target_w = to_wide(target);
    let args_w = to_wide(args);
    let dir_w = to_wide(working_dir);
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR::from_raw(target_w.as_ptr()),
            PCWSTR::from_raw(args_w.as_ptr()),
            PCWSTR::from_raw(dir_w.as_ptr()),
            SW_SHOWNORMAL,
        );
    }
}
