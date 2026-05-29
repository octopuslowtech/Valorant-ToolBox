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
}

impl Config {
    pub fn all_instance_ids(&self) -> Vec<String> {
        self.monitors
            .iter()
            .flat_map(|m| m.instance_ids.clone())
            .collect()
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
