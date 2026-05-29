use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::core::w;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW};

use crate::config::{Config, MonitorSelection};
use crate::constants::APP_NAME;
use crate::lang::{t, Lang};
use crate::monitors::enumerate_monitors;
use crate::paths::config_path;
use crate::worker::{Status, WorkerMsg};
use crate::{blood, config, display};

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

pub struct ToolboxApp {
    resolutions: Vec<String>,
    selected_res: String,
    custom_w: String,
    custom_h: String,
    perf: bool,
    monitors: Vec<MonitorRow>,
    tab: Tab,
    run_on_startup: bool,
    lang: Lang,
    enable_blood: bool,
    enable_vng_remove: bool,
    enable_nvidia_scaling: bool,
    log_lines: Vec<String>,
    status: Status,
    busy: bool,
    rx: Option<Receiver<WorkerMsg>>,
    _tray: Option<TrayIcon>,
    tray_show: Arc<AtomicBool>,
    quitting: bool,
    gpu_names: Vec<String>,
    gpu_is_amd: bool,
}

impl ToolboxApp {
    fn new(ctx: &egui::Context) -> Self {
        let resolutions: Vec<String> = vec![
            "1440x1080  \u{2014}  4:3".to_string(),
            "1280x960  \u{2014}  4:3".to_string(),
            "1024x768  \u{2014}  4:3 classic".to_string(),
            "Custom".to_string(),
        ];

        let saved = config::load_config(&config_path());

        let selected_res = saved
            .as_ref()
            .and_then(|c| {
                let saved_key = format!("{}x{}", c.x, c.y);
                if !c.custom_w.is_empty() && !c.custom_h.is_empty()
                    && c.x == c.custom_w && c.y == c.custom_h
                {
                    return Some("Custom".to_string());
                }
                resolutions.iter().find(|r| r.starts_with(&saved_key)).cloned()
            })
            .unwrap_or_else(|| resolutions[0].clone());
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

        let feat = saved.unwrap_or_else(Config::default_features);

        let gpu_names = crate::process::gpu_names();
        let gpu_is_amd = gpu_names.iter().any(|n| {
            let lower = n.to_lowercase();
            lower.contains("amd") || lower.contains("radeon")
        });

        let tray_show = Arc::new(AtomicBool::new(false));
        let tray = build_tray(Lang::from_str(&feat.language), ctx, tray_show.clone());

        ToolboxApp {
            resolutions,
            selected_res,
            custom_w: feat.custom_w,
            custom_h: feat.custom_h,
            perf: feat.perf,
            monitors,
            tab: Tab::Overview,
            run_on_startup: crate::startup::is_startup_enabled(),
            lang: Lang::from_str(&feat.language),
            enable_blood: feat.enable_blood,
            enable_vng_remove: feat.enable_vng_remove,
            enable_nvidia_scaling: feat.enable_nvidia_scaling,
            log_lines: Vec::new(),
            status: Status::Idle,
            busy: false,
            rx: None,
            _tray: tray,
            tray_show,
            quitting: false,
            gpu_names,
            gpu_is_amd,
        }
    }

    fn build_config(&self) -> Config {
        let res_part = self.selected_res.split("  ").next().unwrap_or(&self.selected_res);
        let parts: Vec<&str> = res_part.split('x').collect();
        let (x, y) = if self.selected_res == "Custom" {
            (self.custom_w.clone(), self.custom_h.clone())
        } else {
            (
                parts.first().copied().unwrap_or("1440").to_string(),
                parts.get(1).copied().unwrap_or("1080").to_string(),
            )
        };
        Config {
            x,
            y,
            perf: self.perf,
            monitors: self
                .monitors
                .iter()
                .filter(|m| m.checked)
                .map(|m| MonitorSelection {
                    name: m.name.clone(),
                    instance_ids: m.instance_ids.clone(),
                })
                .collect(),
            enable_blood: self.enable_blood,
            enable_vng_remove: self.enable_vng_remove,
            enable_nvidia_scaling: self.enable_nvidia_scaling,
            language: self.lang.as_str().to_string(),
            minimize_to_tray: true,
            graphics_preset: "low".to_string(),
            custom_w: self.custom_w.clone(),
            custom_h: self.custom_h.clone(),
        }
    }


    fn start_play(&mut self) {
        if self.busy {
            return;
        }
        let cfg = self.build_config();
        let _ = config::save_config(&config_path(), &cfg);

        let (tx, rx) = channel();
        self.rx = Some(rx);
        self.busy = true;
        std::thread::spawn(move || {
            crate::worker::run(cfg, tx);
        });
    }

    fn revert_resolution(&mut self) {
        let session_path = crate::paths::session_data_path();
        if let Some(data) = config::load_session(&session_path) {
            display::set_resolution(data.x as u32, data.y as u32, data.hz);
            self.log_lines.push(format!("Reverted to {}x{} @ {}hz", data.x, data.y, data.hz));
            let _ = std::fs::remove_file(&session_path);
        } else {
            let (w, h) = display::current_resolution();
            self.log_lines.push(format!("No session data \u{2014} current: {}x{}", w, h));
        }

        let cfg = self.build_config();
        let ids = cfg.all_instance_ids();
        if !ids.is_empty() {
            crate::installer::enable_monitors(&ids);
            self.log_lines.push(format!("Re-enabled {} monitor(s)", ids.len()));
        }

        self.status = Status::Idle;
        self.busy = false;
        self.rx = None;
    }


    fn drain_worker(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.rx {
            let msgs: Vec<WorkerMsg> = rx.try_iter().collect();
            for msg in msgs {
                match msg {
                    WorkerMsg::Log(line) => self.log_lines.push(line),
                    WorkerMsg::SetStatus(s) => {
                        if matches!(s, Status::Idle | Status::Done | Status::Error(_)) {
                            done = true;
                        }
                        self.status = s;
                    }
                }
            }
        }
        if self.log_lines.len() > 500 {
            let excess = self.log_lines.len() - 500;
            self.log_lines.drain(0..excess);
        }
        if done {
            self.busy = false;
            self.rx = None;
        }
    }



}

fn load_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let bytes = include_bytes!("../redyellow.ico");
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = image.dimensions();
    Some((image.into_raw(), w, h))
}

fn build_tray(lang: Lang, ctx: &egui::Context, show_flag: Arc<AtomicBool>) -> Option<TrayIcon> {
    let menu = Menu::new();
    let quit_item = MenuItem::new(t(lang, "tray_quit"), true, None);
    let quit_id = quit_item.id().0.clone();
    if menu.append(&quit_item).is_err() {
        return None;
    }

    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id.0 == quit_id {
            blood::emergency_cleanup();
            std::process::exit(0);
        }
    }));

    let ctx_tray = ctx.clone();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, w!("Valorant-ToolBox")) {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
            show_flag.store(true, Ordering::Relaxed);
            ctx_tray.request_repaint();
        }
    }));

    let icon = load_rgba()
        .and_then(|(rgba, w, h)| tray_icon::Icon::from_rgba(rgba, w, h).ok());

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip(APP_NAME);
    if let Some(icon) = icon {
        builder = builder.with_icon(icon);
    }
    builder.build().ok()
}

impl eframe::App for ToolboxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_worker();

        if self.tray_show.swap(false, Ordering::Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, w!("Valorant-ToolBox")) {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
        }

        if self.quitting {
            blood::emergency_cleanup();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }

        egui::TopBottomPanel::bottom("play_panel")
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 4.0)))
            .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(70.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log_lines {
                        ui.label(egui::RichText::new(line).monospace().size(11.0));
                    }
                });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if self.gpu_names.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, "GPU: Unknown");
                } else if self.gpu_is_amd {
                    let name = self.gpu_names.join(", ");
                    ui.colored_label(egui::Color32::from_rgb(220, 50, 50), format!("GPU: {} — Not supported (AMD)", name));
                } else {
                    let name = self.gpu_names.join(", ");
                    ui.colored_label(egui::Color32::from_rgb(50, 200, 80), format!("GPU: {}", name));
                }
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label("Telegram:");
                ui.hyperlink_to("@anonymususer000012", "https://t.me/anonymususer000012");
            });
            ui.add_space(2.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Overview, t(self.lang, "tab_overview"));
                ui.selectable_value(&mut self.tab, Tab::Advanced, t(self.lang, "tab_advanced"));
            });
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                match self.tab {
                    Tab::Overview => self.ui_overview(ui),
                    Tab::Advanced => self.ui_advanced(ui),
                }
            });
        });

        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

impl ToolboxApp {
    fn ui_overview(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new(t(self.lang, "stretch_title")).strong().size(15.0));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label(t(self.lang, "select_res"));
                    egui::ComboBox::from_id_salt("res_combo")
                        .selected_text(&self.selected_res)
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            for res in &self.resolutions.clone() {
                                ui.selectable_value(&mut self.selected_res, res.clone(), res);
                            }
                        });
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if self.selected_res == "Custom" {
                        ui.label("W:");
                        ui.add(egui::TextEdit::singleline(&mut self.custom_w).desired_width(60.0));
                        ui.label("H:");
                        ui.add(egui::TextEdit::singleline(&mut self.custom_h).desired_width(60.0));
                        ui.add_space(8.0);
                    }

                    let apply_btn = egui::Button::new(
                        egui::RichText::new(t(self.lang, "stretch_apply")).color(egui::Color32::WHITE)
                    )
                        .fill(egui::Color32::from_rgb(40, 140, 60))
                        .min_size(egui::vec2(90.0, 28.0));
                    if ui.add_enabled(!self.busy, apply_btn).clicked() {
                        self.start_play();
                    }

                    let revert_btn = egui::Button::new(
                        egui::RichText::new(t(self.lang, "stretch_revert")).color(egui::Color32::WHITE)
                    )
                        .fill(egui::Color32::from_rgb(140, 60, 40))
                        .min_size(egui::vec2(80.0, 28.0));
                    if ui.add_enabled(!self.busy, revert_btn).clicked() {
                        self.revert_resolution();
                    }
                });

                if self.busy {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.colored_label(egui::Color32::from_rgb(100, 180, 255), "Applying...");
                    });
                }
            });

        if !self.monitors.is_empty() {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(t(self.lang, "tab_advanced")).strong().size(13.0));
            ui.add_space(4.0);
            for mon in &mut self.monitors {
                ui.checkbox(&mut mon.checked, &mon.name);
            }
        }
    }


    fn ui_advanced(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(t(self.lang, "language"));
            let mut is_vi = self.lang == Lang::Vi;
            if ui.selectable_label(!is_vi, "English").clicked() {
                self.lang = Lang::En;
            }
            if ui.selectable_label(is_vi, "Tieng Viet").clicked() {
                self.lang = Lang::Vi;
                is_vi = true;
            }
            let _ = is_vi;
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(6.0);
        if ui.checkbox(&mut self.run_on_startup, t(self.lang, "startup")).changed() {
            crate::startup::set_startup(self.run_on_startup);
        }
        ui.colored_label(egui::Color32::GRAY, t(self.lang, "startup_hint"));
    }
}

fn load_icon() -> Option<egui::IconData> {
    let (rgba, width, height) = load_rgba()?;
    Some(egui::IconData {
        rgba,
        width,
        height,
    })
}

pub fn run() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([480.0, 340.0])
        .with_resizable(true);
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(ToolboxApp::new(&cc.egui_ctx)))),
    )
}
