use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_NORESET,
    CDS_TEST, CDS_TYPE, CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW,
    DISPLAY_DEVICE_PRIMARY_DEVICE, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT,
    DM_PELSWIDTH, ENUM_CURRENT_SETTINGS,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub fn set_dpi_aware() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn current_resolution() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

fn new_devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

pub fn current_refresh_rate() -> u32 {
    unsafe {
        let mut dm = new_devmode();
        if EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
            let hz = dm.dmDisplayFrequency;
            if hz > 0 {
                return hz;
            }
        }
    }
    60
}


fn primary_device_name() -> Option<Vec<u16>> {
    unsafe {
        let mut i = 0u32;
        loop {
            let mut dd = DISPLAY_DEVICEW {
                cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..Default::default()
            };
            if !EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0).as_bool() {
                break;
            }
            if dd.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0 {
                return Some(dd.DeviceName.to_vec());
            }
            i += 1;
        }
    }
    None
}

pub fn resolution_supported(width: u32, height: u32) -> bool {
    unsafe {
        let mut dm = new_devmode();
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        let result = ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_TEST, None);
        result == DISP_CHANGE_SUCCESSFUL
    }
}

pub fn register_custom_resolution(width: u32, height: u32) -> bool {
    let hz = current_refresh_rate();
    unsafe {
        let mut dm = new_devmode();
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        dm.dmDisplayFrequency = hz;
        let flags = CDS_TYPE(CDS_UPDATEREGISTRY.0 | CDS_NORESET.0);
        let result = ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, flags, None);
        result == DISP_CHANGE_SUCCESSFUL
    }
}

pub fn set_resolution(width: u32, height: u32, hz: u32) -> bool {
    unsafe {
        let mut dm = new_devmode();
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        dm.dmDisplayFrequency = if hz > 0 { hz } else { 60 };

        let ok = match primary_device_name() {
            Some(mut name) => {
                let result = ChangeDisplaySettingsExW(
                    PCWSTR::from_raw(name.as_mut_ptr()),
                    Some(&dm),
                    None,
                    CDS_UPDATEREGISTRY,
                    None,
                );
                result == DISP_CHANGE_SUCCESSFUL
            }
            None => {
                let result =
                    ChangeDisplaySettingsExW(PCWSTR::null(), Some(&dm), None, CDS_UPDATEREGISTRY, None);
                result == DISP_CHANGE_SUCCESSFUL
            }
        };

        ok
    }
}
