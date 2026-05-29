#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    En,
    Vi,
}

impl Lang {
    pub fn from_str(s: &str) -> Lang {
        match s {
            "vi" => Lang::Vi,
            _ => Lang::En,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Vi => "vi",
        }
    }
}

pub fn t(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        (Lang::En, "tab_overview") => "Overview",
        (Lang::Vi, "tab_overview") => "Tong quan",
        (Lang::En, "tab_advanced") => "Advanced",
        (Lang::Vi, "tab_advanced") => "Nang cao",
        (Lang::En, "tab_mods") => "Mods",
        (Lang::Vi, "tab_mods") => "Mods",
        (Lang::En, "tab_perf") => "Performance",
        (Lang::Vi, "tab_perf") => "Hieu nang",

        (Lang::En, "custom_res_warn") => "\u{26a0} Custom Resolutions Don't Work",
        (Lang::Vi, "custom_res_warn") => "\u{26a0} Do phan giai tuy chinh khong hoat dong",
        (Lang::En, "select_res") => "Select a stretch resolution:",
        (Lang::Vi, "select_res") => "Chon do phan giai keo gian:",
        (Lang::En, "apply_perf") => "Apply Performance Upgrade",
        (Lang::Vi, "apply_perf") => "Ap dung nang cap hieu nang",
        (Lang::En, "disable_monitors") => "Disable these monitors before launching Valorant:",
        (Lang::Vi, "disable_monitors") => "Tat cac man hinh nay truoc khi mo Valorant:",
        (Lang::En, "disable_monitors_hint") => "Prevents Valorant from hard-locking to 16:9 aspect ratio",
        (Lang::Vi, "disable_monitors_hint") => "Ngan Valorant khoa cung ty le 16:9",
        (Lang::En, "no_monitors") => "No monitors found in Device Manager.",
        (Lang::Vi, "no_monitors") => "Khong tim thay man hinh trong Device Manager.",
        (Lang::En, "install_apply") => "Install & Apply",
        (Lang::Vi, "install_apply") => "Cai dat & Ap dung",
        (Lang::En, "uninstall") => "Uninstall",
        (Lang::Vi, "uninstall") => "Go cai dat",
        (Lang::En, "recovery_data") => "Recovery data: Documents",
        (Lang::Vi, "recovery_data") => "Du lieu khoi phuc: Documents",

        (Lang::En, "play") => "\u{25b6} PLAY VALORANT",
        (Lang::Vi, "play") => "\u{25b6} CHOI VALORANT",
        (Lang::En, "playing") => "Launching...",
        (Lang::Vi, "playing") => "Dang khoi dong...",

        (Lang::En, "mods_blood") => "Blood Mod (Show Mature Content)",
        (Lang::Vi, "mods_blood") => "Mod Mau (Hien noi dung Mature)",
        (Lang::En, "mods_vng") => "Remove VNG Logo",
        (Lang::Vi, "mods_vng") => "Xoa Logo VNG",
        (Lang::En, "mods_hint") => "Mods are injected when the game launches and restored when it closes.",
        (Lang::Vi, "mods_hint") => "Mod duoc chen khi game khoi dong va khoi phuc khi dong game.",

        (Lang::En, "perf_nvidia_scaling") => "Auto NVIDIA scaling = Full-screen",
        (Lang::Vi, "perf_nvidia_scaling") => "Tu dong NVIDIA scaling = Full-screen",
        (Lang::En, "perf_apply_tweaks") => "Apply FPS Tweaks (registry)",
        (Lang::Vi, "perf_apply_tweaks") => "Ap dung FPS Tweaks (registry)",
        (Lang::En, "perf_hint") => "Registry tweaks need admin. A restore point is created first.",
        (Lang::Vi, "perf_hint") => "FPS tweaks can quyen admin. Diem khoi phuc duoc tao truoc.",

        (Lang::En, "startup") => "Open on Windows startup",
        (Lang::Vi, "startup") => "Mo khi khoi dong Windows",
        (Lang::En, "startup_hint") => "Automatically open this tool when you sign in to Windows",
        (Lang::Vi, "startup_hint") => "Tu dong mo cong cu khi dang nhap Windows",
        (Lang::En, "language") => "Language:",
        (Lang::Vi, "language") => "Ngon ngu:",

        (Lang::En, "tray_show") => "Show",
        (Lang::Vi, "tray_show") => "Hien",
        (Lang::En, "tray_quit") => "Quit",
        (Lang::Vi, "tray_quit") => "Thoat",

        (Lang::En, "stretch_title") => "TRUE STRETCH",
        (Lang::Vi, "stretch_title") => "TRUE STRETCH",
        (Lang::En, "stretch_apply") => "Apply",
        (Lang::Vi, "stretch_apply") => "Apply",
        (Lang::En, "stretch_revert") => "Revert",
        (Lang::Vi, "stretch_revert") => "Hoan tac",
        _ => "",
    }
}
