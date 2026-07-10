#![windows_subsystem = "windows"]

mod admin;
mod application;
mod domain;
mod infrastructure;
mod presentation;

use domain::config::Config;
use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;

fn get_arg(args: &[String], prefix: &str) -> Option<String> {
    args.iter()
        .find(|a| a.starts_with(prefix))
        .map(|a| a[prefix.len()..].to_string())
}

fn quote_arg(arg: &str) -> String {
    if arg.contains(' ') || arg.contains('"') {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        arg.to_string()
    }
}

fn args_for_elevation(args: &[String]) -> String {
    args.iter()
        .skip(1)
        .map(|a| quote_arg(a))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if !admin::is_admin() {
        admin::elevate(&args_for_elevation(&args));
        return;
    }

    if args.iter().any(|a| a == "--launch") {
        application::launcher::launch_toolbox();
        return;
    }

    if args.iter().any(|a| a == "--uninstall-direct") {
        application::installer::run_uninstall(true);
        return;
    }

    if args.iter().any(|a| a == "--install-direct") {
        let x = get_arg(&args, "--res-x=").unwrap_or_else(|| "1440".into());
        let y = get_arg(&args, "--res-y=").unwrap_or_else(|| "1080".into());
        let raw_monitors = get_arg(&args, "--monitors=").unwrap_or_default();
        let monitors = application::installer::parse_monitors_arg(&raw_monitors);

        let cfg = Config {
            x,
            y,
            monitors,
            ..Config::default_features()
        };
        application::installer::perform_install(&cfg, true);
        return;
    }

    let Some(_single_instance) = acquire_single_instance() else {
        presentation::dialog::info("Valorant-ToolBox", "Valorant-ToolBox is already running.");
        return;
    };

    let _ = presentation::app::run();
}

fn acquire_single_instance() -> Option<windows::Win32::Foundation::HANDLE> {
    unsafe {
        let handle = CreateMutexW(None, true, w!("Global\\ValorantToolBox_SingleInstance")).ok()?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            return None;
        }
        Some(handle)
    }
}
