use std::collections::BTreeMap;

use eframe::egui;

use crate::admin;
use crate::config::{Config, MonitorSelection};
use crate::constants::APP_NAME;
use crate::display;
use crate::installer;
use crate::monitors::enumerate_monitors;

struct MonitorRow {
    name: String,
    instance_ids: Vec<String>,
    checked: bool,
}

#[derive(PartialEq)]
enum Tab {
    Overview,
    Advanced,
}

pub struct SetupApp {
    resolutions: Vec<String>,
    selected_res: String,
    perf: bool,
    monitors: Vec<MonitorRow>,
    no_monitors: bool,
    tab: Tab,
    run_on_startup: bool,
}

impl SetupApp {
    pub fn new() -> Self {
        let (nw, nh) = display::current_resolution();
        let modes = display::enumerate_modes(nw, nh);
        let resolutions: Vec<String> = modes.iter().map(|(w, h)| format!("{}x{}", w, h)).collect();

        let selected_res = if resolutions.iter().any(|r| r == "1440x1080") {
            "1440x1080".to_string()
        } else {
            resolutions
                .first()
                .cloned()
                .unwrap_or_else(|| "1440x1080".to_string())
        };

        let raw = enumerate_monitors();
        let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for m in raw {
            grouped.entry(m.name).or_default().push(m.instance_id);
        }
        let monitors: Vec<MonitorRow> = grouped
            .into_iter()
            .map(|(name, instance_ids)| MonitorRow {
                name,
                instance_ids,
                checked: false,
            })
            .collect();
        let no_monitors = monitors.is_empty();

        SetupApp {
            resolutions,
            selected_res,
            perf: true,
            monitors,
            no_monitors,
            tab: Tab::Overview,
            run_on_startup: crate::startup::is_startup_enabled(),
        }
    }

    fn selected_monitors(&self) -> Vec<MonitorSelection> {
        self.monitors
            .iter()
            .filter(|m| m.checked)
            .map(|m| MonitorSelection {
                name: m.name.clone(),
                instance_ids: m.instance_ids.clone(),
            })
            .collect()
    }

    fn build_config(&self) -> Config {
        let parts: Vec<&str> = self.selected_res.split('x').collect();
        let x = parts.first().copied().unwrap_or("1440").to_string();
        let y = parts.get(1).copied().unwrap_or("1080").to_string();
        Config {
            x,
            y,
            perf: self.perf,
            monitors: self.selected_monitors(),
        }
    }

    fn do_install(&self, ctx: &egui::Context) {
        let cfg = self.build_config();
        if !admin::is_admin() {
            let monitors_arg = cfg
                .monitors
                .iter()
                .map(|m| format!("{}:::{}", m.name, m.instance_ids.join(",")))
                .collect::<Vec<_>>()
                .join("|");
            let args = format!(
                "--install-direct --res-x={} --res-y={} --perf={} --monitors=\"{}\"",
                cfg.x,
                cfg.y,
                if cfg.perf { 1 } else { 0 },
                monitors_arg
            );
            admin::elevate(&args);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if installer::perform_install(&cfg, true) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn do_uninstall(&self, ctx: &egui::Context) {
        if !admin::is_admin() {
            admin::elevate("--uninstall-direct");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        installer::run_uninstall(true);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(format!("{} SETUP", APP_NAME.to_uppercase()));
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Overview, "Overview");
                ui.selectable_value(&mut self.tab, Tab::Advanced, "Advanced");
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            match self.tab {
                Tab::Overview => {
                    ui.colored_label(egui::Color32::from_rgb(230, 150, 0), "\u{26a0} Custom Resolutions Don't Work");
                    ui.label("Select a stretch resolution:");

                    egui::ComboBox::from_id_salt("res_combo")
                        .selected_text(&self.selected_res)
                        .width(160.0)
                        .show_ui(ui, |ui| {
                            for res in &self.resolutions {
                                ui.selectable_value(&mut self.selected_res, res.clone(), res);
                            }
                        });

                    ui.add_space(8.0);
                    ui.checkbox(&mut self.perf, "Apply Performance Upgrade");

                    ui.add_space(6.0);
                    ui.separator();
                    ui.label(egui::RichText::new("Disable these monitors before launching Valorant:").strong());
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "Prevents Valorant from hard-locking to 16:9 aspect ratio",
                    );
                    ui.add_space(4.0);

                    if self.no_monitors {
                        ui.colored_label(egui::Color32::RED, "No monitors found in Device Manager.");
                    } else {
                        for m in &mut self.monitors {
                            ui.checkbox(&mut m.checked, &m.name);
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    if ui.add(egui::Button::new("Install & Apply").min_size(egui::vec2(160.0, 28.0))).clicked() {
                        self.do_install(ctx);
                    }
                    ui.add_space(4.0);
                    if ui.add(egui::Button::new("Uninstall").min_size(egui::vec2(160.0, 24.0))).clicked() {
                        self.do_uninstall(ctx);
                    }

                    ui.add_space(6.0);
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!("Recovery data: Documents\\{}", APP_NAME),
                    );
                }
                Tab::Advanced => {
                    ui.add_space(4.0);
                    if ui.checkbox(&mut self.run_on_startup, "Open on Windows startup").changed() {
                        crate::startup::set_startup(self.run_on_startup);
                    }
                    ui.colored_label(
                        egui::Color32::GRAY,
                        "Automatically open this tool when you sign in to Windows",
                    );
                }
            }
        });
    }
}

pub fn run_setup() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 620.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        &format!("{} Installer", APP_NAME),
        options,
        Box::new(|_cc| Ok(Box::new(SetupApp::new()))),
    )
}
