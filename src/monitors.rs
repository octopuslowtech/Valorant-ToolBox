use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

use crate::constants::MONITOR_CLASS_GUID;

#[derive(Clone)]
pub struct MonitorEntry {
    pub name: String,
    pub instance_id: String,
}

pub fn enumerate_monitors() -> Vec<MonitorEntry> {
    let mut monitors = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let base = r"SYSTEM\CurrentControlSet\Enum\DISPLAY";

    let display_key = match hklm.open_subkey(base) {
        Ok(k) => k,
        Err(_) => return monitors,
    };

    for model_name in display_key.enum_keys().flatten() {
        let model_key = match display_key.open_subkey(&model_name) {
            Ok(k) => k,
            Err(_) => continue,
        };

        for instance_name in model_key.enum_keys().flatten() {
            let inst_key = match model_key.open_subkey(&instance_name) {
                Ok(k) => k,
                Err(_) => continue,
            };

            let class_guid: String = match inst_key.get_value("ClassGUID") {
                Ok(v) => v,
                Err(_) => continue,
            };
            if class_guid.to_lowercase() != MONITOR_CLASS_GUID.to_lowercase() {
                continue;
            }

            let name = match inst_key.get_value::<String, _>("FriendlyName") {
                Ok(raw) => {
                    let last = raw.split(';').next_back().unwrap_or("").trim().to_string();
                    let trimmed = if last.starts_with('(') && last.ends_with(')') && last.len() >= 2 {
                        last[1..last.len() - 1].to_string()
                    } else {
                        last
                    };
                    if trimmed.is_empty() {
                        model_name.clone()
                    } else {
                        trimmed
                    }
                }
                Err(_) => model_name.clone(),
            };

            let instance_id = format!("DISPLAY\\{}\\{}", model_name, instance_name);
            monitors.push(MonitorEntry { name, instance_id });
        }
    }

    monitors
}
