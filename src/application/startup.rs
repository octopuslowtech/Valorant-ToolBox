use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
use winreg::RegKey;

use crate::domain::constants::APP_NAME;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn is_startup_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(RUN_KEY) {
        return key.get_value::<String, _>(APP_NAME).is_ok();
    }
    false
}

pub fn set_startup(enabled: bool) -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE | KEY_QUERY_VALUE) {
        Ok(k) => k,
        Err(_) => return false,
    };
    if enabled {
        let exe = match std::env::current_exe() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => return false,
        };
        key.set_value(APP_NAME, &format!("\"{}\"", exe)).is_ok()
    } else {
        key.delete_value(APP_NAME).is_ok() || !is_startup_enabled()
    }
}
