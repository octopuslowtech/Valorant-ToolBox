#![windows_subsystem = "windows"]

mod admin;
mod config;
mod constants;
mod dialog;
mod display;
mod gui;
mod ini;
mod installer;
mod launcher;
mod logger;
mod monitors;
mod paths;
mod process;
mod riot;
mod shortcut;
mod startup;

use config::Config;

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
        };
        installer::perform_install(&cfg, true);
        return;
    }

    let _ = gui::run_setup();
}
