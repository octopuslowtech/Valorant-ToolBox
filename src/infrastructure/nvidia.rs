use winreg::enums::*;
use winreg::RegKey;

fn set_scaling_recursive(key: &RegKey, count: &mut u32) {
    if key.get_value::<u32, _>("Scaling").is_ok() {
        if key.set_value("Scaling", &3u32).is_ok() {
            *count += 1;
        }
    }
    let subkey_names: Vec<String> = key.enum_keys().flatten().collect();
    for name in subkey_names {
        if let Ok(sub) = key.open_subkey_with_flags(&name, KEY_READ | KEY_SET_VALUE) {
            set_scaling_recursive(&sub, count);
        }
    }
}

pub fn set_scaling_fullscreen() -> (bool, String) {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let path = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers\Configuration";
    let root = match hklm.open_subkey_with_flags(path, KEY_READ | KEY_SET_VALUE) {
        Ok(k) => k,
        Err(_) => return (false, "NVIDIA scaling: config key not found (needs admin?)".into()),
    };
    let mut count = 0u32;
    set_scaling_recursive(&root, &mut count);
    if count > 0 {
        (true, format!("NVIDIA scaling set to Full-screen ({} display(s))", count))
    } else {
        (false, "No NVIDIA scaling keys found".into())
    }
}
