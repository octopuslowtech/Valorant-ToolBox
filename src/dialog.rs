use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn show(title: &str, text: &str, style: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE) {
    let title_w = to_wide(title);
    let text_w = to_wide(text);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(text_w.as_ptr()),
            PCWSTR::from_raw(title_w.as_ptr()),
            MB_OK | style,
        );
    }
}

pub fn info(title: &str, text: &str) {
    show(title, text, MB_ICONINFORMATION);
}

pub fn warn(title: &str, text: &str) {
    show(title, text, MB_ICONWARNING);
}

pub fn error(title: &str, text: &str) {
    show(title, text, MB_ICONERROR);
}
