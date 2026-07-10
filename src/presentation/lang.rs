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
        (Lang::En, "startup") => "Open on Windows startup",
        (Lang::Vi, "startup") => "Mo khi khoi dong Windows",
        (Lang::En, "language") => "Language:",
        (Lang::Vi, "language") => "Ngon ngu:",

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
