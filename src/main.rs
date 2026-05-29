#![windows_subsystem = "windows"]

mod admin;
mod app;
mod blood;
mod config;
mod constants;
mod dialog;
mod display;
mod ini;
mod installer;
mod lang;
mod launcher;
mod logger;
mod monitors;
mod nvidia;
mod paths;
mod process;
mod riot;
mod shortcut;
mod startup;
mod worker;

use config::Config;
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;

fn get_arg(args: &[String], prefix: &str) -> Option<String> {
    args.iter()
        .find(|a| a.starts_with(prefix))
        .map(|a| a[prefix.len()..].to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--launch") {
        launcher::launch_toolbox();
        return;
    }

    if args.iter().any(|a| a == "--uninstall-direct") {
        installer::run_uninstall(true);
        return;
    }

    if args.iter().any(|a| a == "--install-direct") {
        let x = get_arg(&args, "--res-x=").unwrap_or_else(|| "1440".into());
        let y = get_arg(&args, "--res-y=").unwrap_or_else(|| "1080".into());
        let perf = get_arg(&args, "--perf=").unwrap_or_else(|| "1".into()) != "0";
        let raw_monitors = get_arg(&args, "--monitors=").unwrap_or_default();
        let monitors = installer::parse_monitors_arg(&raw_monitors);

        let cfg = Config {
            x,
            y,
            perf,
            monitors,
            ..Config::default_features()
        };
        installer::perform_install(&cfg, true);
        return;
    }

    if !admin::is_admin() {
        admin::elevate("");
        return;
    }

    if !acquire_single_instance() {
        dialog::info("Valorant-ToolBox", "Valorant-ToolBox is already running.");
        return;
    }

    let _ = app::run();
}

fn acquire_single_instance() -> bool {
    unsafe {
        let handle = CreateMutexW(None, true, w!("Global\\ValorantToolBox_SingleInstance"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(h) = handle {
                let _ = CloseHandle(h);
            }
            return false;
        }
        true
    }
}
