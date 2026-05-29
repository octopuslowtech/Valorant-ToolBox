use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::core::w;
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW};

use crate::domain::config::{Config, MonitorSelection};
use crate::domain::constants::APP_NAME;
use crate::presentation::lang::{t, Lang};
use crate::infrastructure::paths::config_path;
use crate::infrastructure::vibrance::VibranceState;
use crate::application::worker::{Status, WorkerMsg};
use crate::infrastructure::{blood, display};
use crate::domain::config;



#[derive(PartialEq)]
enum Tab {
    Overview,
    Performance,
    Advanced,
}

pub struct ToolboxApp {
    resolutions: Vec<String>,
    selected_res: String,
    custom_w: String,
    custom_h: String,
    perf: bool,
    run_on_startup: bool,
    lang: Lang,
    enable_blood: bool,
    enable_vng_remove: bool,
    enable_nvidia_scaling: bool,
    status: Status,
    busy: bool,
    rx: Option<Receiver<WorkerMsg>>,
    _tray: Option<TrayIcon>,
    tray_show: Arc<AtomicBool>,
    quitting: bool,
    gpu_names: Vec<String>,
    gpu_is_amd: bool,
    vibrance: VibranceState,
    vibrance_level: i32,
    tab: Tab,
    optimize_log: Vec<String>,
    optimizing: bool,
    optimize_rx: Option<Receiver<String>>,
    optimize_done: bool,
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
                if !c.selected_preset.is_empty() {
                    return Some(c.selected_preset.clone());
                }
                let saved_key = format!("{}x{}", c.x, c.y);
                if !c.custom_w.is_empty() && !c.custom_h.is_empty()
                    && c.x == c.custom_w && c.y == c.custom_h
                {
                    return Some("Custom".to_string());
                }
                resolutions.iter().find(|r| r.starts_with(&saved_key)).cloned()
            })
            .unwrap_or_else(|| resolutions[0].clone());

        let feat = saved.unwrap_or_else(Config::default_features);

        let gpu_names = crate::infrastructure::process::gpu_names();
        let gpu_is_amd = gpu_names.iter().any(|n| {
            let lower = n.to_lowercase();
            lower.contains("amd") || lower.contains("radeon")
        });

        let tray_show = Arc::new(AtomicBool::new(false));
        let tray = build_tray(Lang::from_str(&feat.language), ctx, tray_show.clone());

        let vibrance = VibranceState::new(gpu_is_amd);
        let vibrance_level = feat.vibrance_level;

        if vibrance_level != 50 {
            let nv_level = map_percent_to_level(vibrance_level, gpu_is_amd);
            vibrance.ingame_level.store(nv_level, Ordering::Relaxed);
            vibrance.start();
        }

        ToolboxApp {
            resolutions,
            selected_res,
            custom_w: feat.custom_w,
            custom_h: feat.custom_h,
            perf: feat.perf,
            run_on_startup: crate::application::startup::is_startup_enabled(),
            lang: Lang::from_str(&feat.language),
            enable_blood: feat.enable_blood,
            enable_vng_remove: feat.enable_vng_remove,
            enable_nvidia_scaling: feat.enable_nvidia_scaling,
            status: Status::Idle,
            busy: false,
            rx: None,
            _tray: tray,
            tray_show,
            quitting: false,
            gpu_names,
            gpu_is_amd,
            vibrance,
            vibrance_level,
            tab: Tab::Overview,
            optimize_log: Vec::new(),
            optimizing: false,
            optimize_rx: None,
            optimize_done: false,
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
            monitors: {
                let raw = crate::infrastructure::monitors::enumerate_monitors();
                let mut grouped: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
                for m in raw {
                    grouped.entry(m.name).or_default().push(m.instance_id);
                }
                grouped.into_iter().map(|(name, instance_ids)| MonitorSelection { name, instance_ids }).collect()
            },
            enable_blood: self.enable_blood,
            enable_vng_remove: self.enable_vng_remove,
            enable_nvidia_scaling: self.enable_nvidia_scaling,
            language: self.lang.as_str().to_string(),
            minimize_to_tray: true,
            graphics_preset: "low".to_string(),
            custom_w: self.custom_w.clone(),
            custom_h: self.custom_h.clone(),
            vibrance_level: self.vibrance_level,
            selected_preset: self.selected_res.clone(),
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
            crate::application::worker::run(cfg, tx);
        });
    }

    fn revert_resolution(&mut self) {
        let session_path = crate::infrastructure::paths::session_data_path();
        if let Some(data) = config::load_session(&session_path) {
            display::set_resolution(data.x as u32, data.y as u32, data.hz);
            let _ = std::fs::remove_file(&session_path);
        }

        let cfg = self.build_config();
        let ids = cfg.all_instance_ids();
        if !ids.is_empty() {
            crate::application::installer::enable_monitors(&ids);
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
                    WorkerMsg::Log(_) => {}
                    WorkerMsg::SetStatus(s) => {
                        if matches!(s, Status::Idle | Status::Done | Status::Error(_)) {
                            done = true;
                        }
                        self.status = s;
                    }
                }
            }
        }
        if done {
            self.busy = false;
            self.rx = None;
        }
    }



}

fn load_rgba() -> Option<(Vec<u8>, u32, u32)> {
    let bytes = include_bytes!("../../redyellow.ico");
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
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(15, 25, 35);
        visuals.window_fill = egui::Color32::from_rgb(15, 25, 35);
        visuals.faint_bg_color = egui::Color32::from_rgb(20, 32, 44);
        visuals.extreme_bg_color = egui::Color32::from_rgb(8, 14, 20);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(20, 32, 44);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 40, 55);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 55, 75);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(255, 70, 85);
        visuals.selection.bg_fill = egui::Color32::from_rgb(255, 70, 85);
        visuals.override_text_color = Some(egui::Color32::from_rgb(230, 237, 243));
        ctx.set_visuals(visuals);

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

        egui::TopBottomPanel::bottom("footer")
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.gpu_names.is_empty() {
                        ui.colored_label(egui::Color32::from_rgb(125, 133, 144), "GPU: Unknown");
                    } else if self.gpu_is_amd {
                        let name = self.gpu_names.join(", ");
                        ui.colored_label(egui::Color32::from_rgb(255, 70, 85), format!("{}  (unsupported)", name));
                    } else {
                        let name = self.gpu_names.join(", ");
                        ui.colored_label(egui::Color32::from_rgb(23, 232, 160), &name);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.hyperlink_to("@anonymususer000012", "https://t.me/anonymususer000012");
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(16.0, 12.0)))
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.tab, Tab::Overview, "Overview");
                    ui.selectable_value(&mut self.tab, Tab::Performance, "Performance");
                    ui.selectable_value(&mut self.tab, Tab::Advanced, "Advanced");
                });
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                match self.tab {
                    Tab::Overview => {
                        self.ui_resolution(ui);
                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(12.0);
                        self.ui_vibrance(ui);
                    }
                    Tab::Performance => {
                        self.ui_performance(ui);
                    }
                    Tab::Advanced => {
                        self.ui_settings(ui);
                    }
                }
            });

        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }
}

impl ToolboxApp {
    fn ui_resolution(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new(t(self.lang, "stretch_title")).strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("res_combo")
                .selected_text(&self.selected_res)
                .width(200.0)
                .show_ui(ui, |ui| {
                    for res in &self.resolutions.clone() {
                        ui.selectable_value(&mut self.selected_res, res.clone(), res);
                    }
                });

            if self.selected_res == "Custom" {
                ui.add_space(8.0);
                ui.add(egui::TextEdit::singleline(&mut self.custom_w).desired_width(50.0).hint_text("W"));
                ui.label("\u{00d7}");
                ui.add(egui::TextEdit::singleline(&mut self.custom_h).desired_width(50.0).hint_text("H"));
            }
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let apply_btn = egui::Button::new(
                egui::RichText::new(t(self.lang, "stretch_apply")).color(egui::Color32::WHITE).strong()
            )
                .fill(egui::Color32::from_rgb(255, 70, 85))
                .rounding(4.0)
                .min_size(egui::vec2(100.0, 30.0));
            if ui.add_enabled(!self.busy, apply_btn).clicked() {
                self.start_play();
            }

            ui.add_space(6.0);
            let revert_btn = egui::Button::new(
                egui::RichText::new(t(self.lang, "stretch_revert")).color(egui::Color32::from_rgb(200, 200, 210))
            )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 90, 110)))
                .rounding(4.0)
                .min_size(egui::vec2(90.0, 30.0));
            if ui.add_enabled(!self.busy, revert_btn).clicked() {
                self.revert_resolution();
            }

            if self.busy {
                ui.add_space(8.0);
                ui.spinner();
            }
        });
        ui.add_space(6.0);
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 60),
            "\u{26a0} Restart game after Apply to take effect",
        );
    }

    fn ui_vibrance(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Digital Vibrance");
            ui.colored_label(
                egui::Color32::from_rgb(125, 133, 144),
                if self.gpu_is_amd { "(AMD)" } else { "(NVIDIA)" },
            );
        });
        ui.add_space(4.0);
        let slider = egui::Slider::new(&mut self.vibrance_level, 50..=100)
            .suffix("%");
        if ui.add(slider).changed() {
            if self.vibrance_level != 50 {
                let nv_level = map_percent_to_level(self.vibrance_level, self.gpu_is_amd);
                self.vibrance.ingame_level.store(nv_level, Ordering::Relaxed);
                self.vibrance.start();
                self.vibrance.apply_immediate();
            } else {
                self.vibrance.stop();
            }
            let cfg = self.build_config();
            let _ = config::save_config(&config_path(), &cfg);
        }
        ui.add_space(6.0);
        ui.colored_label(
            egui::Color32::from_rgb(255, 200, 60),
            "\u{26a0} Auto-applies when Valorant is running",
        );
    }

    fn ui_performance(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Performance Optimization").strong());
        ui.add_space(4.0);
        ui.colored_label(
            egui::Color32::from_rgb(125, 133, 144),
            "Set Valorant CPU priority, power plan, disable services, network tweaks, registry tweaks",
        );
        ui.add_space(10.0);

        if let Some(rx) = &self.optimize_rx {
            let msgs: Vec<String> = rx.try_iter().collect();
            for msg in msgs {
                if msg == "__DONE__" {
                    self.optimizing = false;
                    self.optimize_done = true;
                    self.optimize_rx = None;
                    break;
                }
                self.optimize_log.push(msg);
            }
        }

        if self.optimizing {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Optimizing system...");
            });
            ui.add_space(6.0);
        }

        if !self.optimizing && !self.optimize_done {
            let btn = egui::Button::new(
                egui::RichText::new("Optimize Now").color(egui::Color32::WHITE).strong()
            )
                .fill(egui::Color32::from_rgb(23, 232, 160))
                .rounding(4.0)
                .min_size(egui::vec2(130.0, 32.0));

            if ui.add(btn).clicked() {
                self.optimizing = true;
                self.optimize_done = false;
                self.optimize_log.clear();
                let (tx, rx) = channel::<String>();
                self.optimize_rx = Some(rx);
                std::thread::spawn(move || {
                    crate::infrastructure::optimize::run_all(|msg| {
                        let _ = tx.send(msg);
                    });
                    let _ = tx.send("__DONE__".into());
                });
            }
        }

        if self.optimize_done {
            ui.add_space(6.0);
            let all_ok = self.optimize_log.iter().all(|l| l.contains("[OK]"));
            if all_ok {
                ui.colored_label(
                    egui::Color32::from_rgb(23, 232, 160),
                    "\u{2705} All optimizations applied successfully!",
                );
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 60),
                    "\u{26a0} Some optimizations could not be applied (see details below)",
                );
            }
            ui.add_space(4.0);
            if ui.small_button("Run again").clicked() {
                self.optimize_done = false;
                self.optimize_log.clear();
            }
        }

        if !self.optimize_log.is_empty() {
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for line in &self.optimize_log {
                    let color = if line.contains("[OK]") {
                        egui::Color32::from_rgb(23, 232, 160)
                    } else {
                        egui::Color32::from_rgb(255, 100, 100)
                    };
                    ui.colored_label(color, line);
                }
            });
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        if ui.checkbox(&mut self.run_on_startup, t(self.lang, "startup")).changed() {
            crate::application::startup::set_startup(self.run_on_startup);
        }
        ui.colored_label(
            egui::Color32::from_rgb(125, 133, 144),
            "Automatically open this tool when you sign in to Windows",
        );
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_rgb(125, 133, 144), t(self.lang, "language"));
            ui.add_space(4.0);
            let is_vi = self.lang == Lang::Vi;
            if ui.selectable_label(!is_vi, "EN").clicked() {
                self.lang = Lang::En;
            }
            if ui.selectable_label(is_vi, "VI").clicked() {
                self.lang = Lang::Vi;
            }
        });
    }
}

fn map_percent_to_level(percent: i32, is_amd: bool) -> i32 {
    if is_amd {
        (percent - 50) * 2 + 100
    } else {
        (percent - 50) * 63 / 50
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
        .with_inner_size([460.0, 320.0])
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
