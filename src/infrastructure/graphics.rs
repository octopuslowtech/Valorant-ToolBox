use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::infrastructure::paths::set_read_only;

#[derive(Clone, Copy, PartialEq)]
pub enum GraphicsPreset {
    Low,
    Medium,
    High,
}

impl GraphicsPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            GraphicsPreset::Low => "low",
            GraphicsPreset::Medium => "medium",
            GraphicsPreset::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "medium" => GraphicsPreset::Medium,
            "high" => GraphicsPreset::High,
            _ => GraphicsPreset::Low,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            GraphicsPreset::Low => "Low",
            GraphicsPreset::Medium => "Medium",
            GraphicsPreset::High => "High",
        }
    }

    pub fn all() -> &'static [GraphicsPreset] {
        &[GraphicsPreset::Low, GraphicsPreset::Medium, GraphicsPreset::High]
    }
}

pub fn find_valorant_settings_dir() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let base = Path::new(&local).join("VALORANT").join("Saved").join("Config");
    if base.exists() {
        Some(base)
    } else {
        None
    }
}

pub fn find_all_settings_files() -> Vec<PathBuf> {
    let base = match find_valorant_settings_dir() {
        Some(b) => b,
        None => return Vec::new(),
    };
    WalkDir::new(&base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() == "GameUserSettings.ini")
        .map(|e| e.path().to_path_buf())
        .filter(|p| !p.to_string_lossy().contains("CrashReportClient"))
        .collect()
}

pub fn detect_settings_count() -> usize {
    find_all_settings_files().len()
}

fn scalability_values(preset: GraphicsPreset) -> u8 {
    match preset {
        GraphicsPreset::Low => 0,
        GraphicsPreset::Medium => 1,
        GraphicsPreset::High => 2,
    }
}

fn graphics_quality_value(preset: GraphicsPreset) -> u8 {
    match preset {
        GraphicsPreset::Low => 0,
        GraphicsPreset::Medium => 1,
        GraphicsPreset::High => 2,
    }
}

fn build_preset_content(preset: GraphicsPreset) -> String {
    let sq = scalability_values(preset);
    let gq = graphics_quality_value(preset);
    let (distortion, bloom, shadow, clarity) = match preset {
        GraphicsPreset::Low => ("False", "False", "False", "False"),
        GraphicsPreset::Medium => ("True", "False", "False", "False"),
        GraphicsPreset::High => ("True", "True", "True", "True"),
    };
    let aniso = match preset {
        GraphicsPreset::Low => 0,
        GraphicsPreset::Medium => 2,
        GraphicsPreset::High => 4,
    };

    format!(
        "[/Script/ShooterGame.ShooterGameUserSettings]\n\
         bShouldDisplayCommunityRules=False\n\
         bUseDesiredSafeZone=True\n\
         bUseHDRDisplayOutput=False\n\
         HDRDisplayOutputNits=1000\n\
         bUseDynamicResolution=False\n\
         ResolutionSizeRule=2\n\
         bUseFrontEndScene=True\n\
         EAresNetMode=0\n\
         bSettingOverrideSectionExpanded=False\n\
         \n\
         [ScalabilityGroups]\n\
         sg.ResolutionQuality=100\n\
         sg.ViewDistanceQuality={sq}\n\
         sg.AntiAliasingQuality={sq}\n\
         sg.ShadowQuality={sq}\n\
         sg.GlobalIlluminationQuality={sq}\n\
         sg.ReflectionQuality={sq}\n\
         sg.PostProcessQuality={sq}\n\
         sg.TextureQuality={sq}\n\
         sg.EffectsQuality={sq}\n\
         sg.FoliageQuality={sq}\n\
         sg.ShadingQuality={sq}\n\
         \n\
         [/Script/Engine.GameUserSettings]\n\
         bUseVSync=False\n\
         bUseDynamicResolution=False\n\
         ResolutionSizeRule=2\n\
         LastUserConfirmedResolutionSizeRule=2\n\
         FrameRateLimit=0.000000\n\
         \n\
         [/Script/ShooterGame.ShooterGraphicsUserSettings]\n\
         bDataCenterHintShouldDisplay=True\n\
         GraphicsQuality={gq}\n\
         bShouldLetterbox=False\n\
         MaterialQualityLevel={gq}\n\
         OverallGraphicsQuality={gq}\n\
         bEnableDistortion={distortion}\n\
         bEnableBloom={bloom}\n\
         bEnableFirstPersonShadow={shadow}\n\
         bEnableClarityBoost={clarity}\n\
         MultithreadedRendering=1\n\
         AnisotropicFiltering={aniso}\n\
         bImproveClarity={clarity}\n\
         Sharpening=0.000000\n\
         ExperimentalSharpening=0.000000\n"
    )
}

pub fn apply_preset(preset: GraphicsPreset) -> (bool, String) {
    let files = find_all_settings_files();
    if files.is_empty() {
        return (false, "GameUserSettings.ini not found. Launch VALORANT at least once.".to_string());
    }

    let content = build_preset_content(preset);
    let mut applied = 0;

    for path in &files {
        set_read_only(path, false);
        let backup = path.with_extension("ini.vtb_backup");
        if !backup.exists() {
            let _ = std::fs::copy(path, &backup);
        }
        if std::fs::write(path, &content).is_ok() {
            set_read_only(path, true);
            applied += 1;
        }
    }

    (true, format!("{} preset applied to {} account(s)", preset.label(), applied))
}

pub fn restore_settings() -> (bool, String) {
    let files = find_all_settings_files();
    if files.is_empty() {
        return (false, "GameUserSettings.ini not found.".to_string());
    }

    let mut restored = 0;
    for path in &files {
        let backup = path.with_extension("ini.vtb_backup");
        if backup.exists() {
            set_read_only(path, false);
            if std::fs::copy(&backup, path).is_ok() {
                set_read_only(path, true);
                restored += 1;
            }
        }
    }

    if restored == 0 {
        (false, "No backups found.".to_string())
    } else {
        (true, format!("Restored {} account(s)", restored))
    }
}
