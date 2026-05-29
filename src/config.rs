use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct MonitorSelection {
    pub name: String,
    pub instance_ids: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub x: String,
    pub y: String,
    pub perf: bool,
    #[serde(default)]
    pub monitors: Vec<MonitorSelection>,
    #[serde(default = "default_true")]
    pub enable_blood: bool,
    #[serde(default = "default_true")]
    pub enable_vng_remove: bool,
    #[serde(default = "default_true")]
    pub enable_nvidia_scaling: bool,
    #[serde(default = "default_language")]
    pub language: String,
#[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_graphics_preset")]
    pub graphics_preset: String,
    #[serde(default)]
    pub custom_w: String,
    #[serde(default)]
    pub custom_h: String,
    #[serde(default = "default_vibrance_level")]
    pub vibrance_level: i32,
}

impl Config {
    pub fn all_instance_ids(&self) -> Vec<String> {
        self.monitors
            .iter()
            .flat_map(|m| m.instance_ids.clone())
            .collect()
    }

    pub fn default_features() -> Config {
        Config {
            x: "1440".to_string(),
            y: "1080".to_string(),
            perf: true,
            monitors: Vec::new(),
            enable_blood: true,
            enable_vng_remove: true,
            enable_nvidia_scaling: true,
            language: "en".to_string(),
            minimize_to_tray: true,
            graphics_preset: "low".to_string(),
            custom_w: String::new(),
            custom_h: String::new(),
            vibrance_level: 50,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub x: i32,
    pub y: i32,
    pub hz: u32,
}

pub fn load_config(path: &std::path::Path) -> Option<Config> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_config(path: &std::path::Path, cfg: &Config) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, text)
}

pub fn load_session(path: &std::path::Path) -> Option<SessionData> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_session(path: &std::path::Path, data: &SessionData) -> std::io::Result<()> {
    let text = serde_json::to_string(data)?;
    std::fs::write(path, text)
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en".to_string()
}

fn default_graphics_preset() -> String {
    "low".to_string()
}

fn default_vibrance_level() -> i32 {
    50
}
