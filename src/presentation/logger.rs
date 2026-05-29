use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Logger {
    path: PathBuf,
}

fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs = now % 60;
    let mins = (now / 60) % 60;
    let hours = (now / 3600) % 24;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

impl Logger {
    pub fn create(path: PathBuf) -> Self {
        if let Ok(mut f) = std::fs::File::create(&path) {
            let _ = writeln!(f, "=== Valorant-ToolBox Debug ===\n");
        }
        Logger { path }
    }

    pub fn log(&self, msg: &str) {
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&self.path) {
            let _ = writeln!(f, "[{}] {}", timestamp(), msg);
        }
    }
}
