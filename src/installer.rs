use crate::config::{Config, MonitorSelection};
use crate::dialog;
use crate::display;
use crate::paths::{config_path, ensure_data_folder, valorant_config_root};
use crate::process::{has_nvidia_gpu, pnputil_disable, pnputil_enable};
use crate::shortcut::{create_shortcut, remove_shortcut};
use crate::{config, constants::APP_NAME, ini};

const NVIDIA_MESSAGE: &str = "To prevent black bars in Valorant you must enable GPU scaling override:\n\n\
1. Right-click your desktop -> NVIDIA Control Panel\n\
2. Click 'Adjust desktop size and position'\n\
3. Set Scaling to 'Full-screen'\n\
4. Check 'Override the scaling mode set by games and programs'\n\
5. Click Apply\n\n\
This is a one-time setup step and only needs to be done once.";

pub fn check_nvidia_scaling() {
    if !has_nvidia_gpu() {
        return;
    }
    dialog::info("NVIDIA GPU Detected - Action Required", NVIDIA_MESSAGE);
}

pub fn disable_monitors(ids: &[String]) {
    for id in ids {
        pnputil_disable(id);
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
}

pub fn enable_monitors(ids: &[String]) {
    for id in ids {
        pnputil_enable(id);
    }
    std::thread::sleep(std::time::Duration::from_secs(1));
}

pub fn perform_install(cfg: &Config, show_dialogs: bool) -> bool {
    ensure_data_folder();

    let width: u32 = cfg.x.parse().unwrap_or(0);
    let height: u32 = cfg.y.parse().unwrap_or(0);

    if width == 0 || height == 0 || !display::resolution_supported(width, height) {
        if show_dialogs {
            dialog::error(
                "Unsupported Resolution",
                &format!(
                    "{}x{} is not supported by your display driver.\n\n\
                     Please try a different resolution. Common stretched resolutions:\n\
                     \u{2022} 1440x1080\n\
                     \u{2022} 1280x1080\n\
                     \u{2022} 1024x768\n\n\
                     If you need a custom resolution, add it first via\n\
                     NVIDIA Control Panel -> Change Resolution -> Customize.",
                    cfg.x, cfg.y
                ),
            );
        }
        return false;
    }

    if let Err(e) = config::save_config(&config_path(), cfg) {
        if show_dialogs {
            dialog::error("Error", &format!("Setup failed: {}", e));
        }
        return false;
    }

    let root = valorant_config_root();
    ini::run_installation(&root, &cfg.x, &cfg.y, cfg.perf);
    display::register_custom_resolution(width, height);
    check_nvidia_scaling();

    let ids = cfg.all_instance_ids();
    if !ids.is_empty() {
        disable_monitors(&ids);
    }

    let shortcut_ok = create_shortcut();

    if show_dialogs {
        let mon_lines = if cfg.monitors.is_empty() {
            "  (none - monitors will stay enabled)".to_string()
        } else {
            cfg.monitors
                .iter()
                .map(|m| format!("  \u{2022} {}", m.name))
                .collect::<Vec<_>>()
                .join("\n")
        };
        if shortcut_ok {
            dialog::info(
                "Success",
                &format!(
                    "{} is ready!\n\nMonitors disabled at launch:\n{}\n\nRecovery data: Documents\\{}",
                    APP_NAME, mon_lines, APP_NAME
                ),
            );
        } else {
            dialog::warn(
                APP_NAME,
                "Installation complete but the desktop shortcut could not be created.\n\
                 You can run StretchyVal.exe directly with the --launch argument.",
            );
        }
    }

    true
}

pub fn run_uninstall(show_dialog: bool) {
    let mut monitors_enabled = 0;
    if let Some(cfg) = config::load_config(&config_path()) {
        let ids = cfg.all_instance_ids();
        if !ids.is_empty() {
            enable_monitors(&ids);
            monitors_enabled = ids.len();
        }
    }

    let root = valorant_config_root();
    let unlocked = ini::unlock_all_inis(&root);

    let _ = std::fs::remove_file(config_path());
    remove_shortcut();

    if show_dialog {
        dialog::info(
            "Uninstalled",
            &format!(
                "{} has been uninstalled.\n\n\
                 \u{2022} {} monitor(s) re-enabled\n\
                 \u{2022} {} Valorant config file(s) unlocked\n\
                 \u{2022} Desktop shortcut removed\n\
                 \u{2022} Config file deleted\n\n\
                 Your Valorant settings are now fully editable again.\n\
                 Launch Valorant from the official shortcut to restore your preferred settings.",
                APP_NAME, monitors_enabled, unlocked
            ),
        );
    }
}

pub fn parse_monitors_arg(raw: &str) -> Vec<MonitorSelection> {
    let mut selected = Vec::new();
    if raw.is_empty() {
        return selected;
    }
    for entry in raw.split('|') {
        if let Some((name, ids_str)) = entry.split_once(":::") {
            let ids = ids_str
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            selected.push(MonitorSelection {
                name: name.to_string(),
                instance_ids: ids,
            });
        }
    }
    selected
}
