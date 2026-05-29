#![allow(non_camel_case_types, dead_code)]

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, MSG,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};
use windows::core::PCSTR;

const NVAPI_MAX_PHYSICAL_GPUS: usize = 64;

type NvAPI_QueryInterface_t = unsafe extern "C" fn(u32) -> *const ();
type NvAPI_Initialize_t = unsafe extern "C" fn() -> i32;
type NvAPI_EnumNvidiaDisplayHandle_t = unsafe extern "C" fn(i32, *mut i32) -> i32;
type NvAPI_SetDVCLevel_t = unsafe extern "C" fn(i32, i32, i32) -> i32;
type NvAPI_GetDVCInfo_t = unsafe extern "C" fn(i32, i32, *mut NvDVCInfo) -> i32;

#[repr(C)]
struct NvDVCInfo {
    version: u32,
    current_level: i32,
    min_level: i32,
    max_level: i32,
}

const NVAPI_INITIALIZE_ID: u32 = 0x0150E828;
const NVAPI_ENUM_DISPLAY_HANDLE_ID: u32 = 0x9ABDD40D;
const NVAPI_SET_DVC_LEVEL_ID: u32 = 0x172409B4;
const NVAPI_GET_DVC_INFO_ID: u32 = 0x4085DE45;

struct NvidiaApi {
    initialize: NvAPI_Initialize_t,
    enum_display: NvAPI_EnumNvidiaDisplayHandle_t,
    set_dvc: NvAPI_SetDVCLevel_t,
    get_dvc: NvAPI_GetDVCInfo_t,
    display_handle: i32,
}

impl NvidiaApi {
    fn load() -> Option<Self> {
        unsafe {
            let dll = LoadLibraryA(PCSTR(b"nvapi64.dll\0".as_ptr())).ok()?;
            let query_fn = GetProcAddress(dll, PCSTR(b"nvapi_QueryInterface\0".as_ptr()))?;
            let query: NvAPI_QueryInterface_t = std::mem::transmute(query_fn);

            let init_ptr = query(NVAPI_INITIALIZE_ID);
            if init_ptr.is_null() { return None; }
            let initialize: NvAPI_Initialize_t = std::mem::transmute(init_ptr);

            let enum_ptr = query(NVAPI_ENUM_DISPLAY_HANDLE_ID);
            if enum_ptr.is_null() { return None; }
            let enum_display: NvAPI_EnumNvidiaDisplayHandle_t = std::mem::transmute(enum_ptr);

            let set_ptr = query(NVAPI_SET_DVC_LEVEL_ID);
            if set_ptr.is_null() { return None; }
            let set_dvc: NvAPI_SetDVCLevel_t = std::mem::transmute(set_ptr);

            let get_ptr = query(NVAPI_GET_DVC_INFO_ID);
            if get_ptr.is_null() { return None; }
            let get_dvc: NvAPI_GetDVCInfo_t = std::mem::transmute(get_ptr);

            if initialize() != 0 { return None; }

            let mut handle: i32 = 0;
            if enum_display(0, &mut handle) != 0 { return None; }

            Some(NvidiaApi {
                initialize,
                enum_display,
                set_dvc,
                get_dvc,
                display_handle: handle,
            })
        }
    }

    fn set_level(&self, level: i32) -> bool {
        unsafe { (self.set_dvc)(self.display_handle, 0, level) == 0 }
    }

    fn get_level(&self) -> Option<i32> {
        unsafe {
            let mut info = NvDVCInfo {
                version: (std::mem::size_of::<NvDVCInfo>() as u32) | (1 << 16),
                current_level: 0,
                min_level: 0,
                max_level: 0,
            };
            if (self.get_dvc)(self.display_handle, 0, &mut info) == 0 {
                Some(info.current_level)
            } else {
                None
            }
        }
    }
}

type ADL_Main_Control_Create_t = unsafe extern "C" fn(
    extern "C" fn(i32) -> *mut std::ffi::c_void,
    i32,
) -> i32;
type ADL_Main_Control_Destroy_t = unsafe extern "C" fn() -> i32;
type ADL2_Display_Color_Set_t = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    i32,
    i32,
    i32,
    i32,
) -> i32;
type ADL_Adapter_NumberOfAdapters_Get_t = unsafe extern "C" fn(*mut i32) -> i32;
type ADL_Display_Color_Set_t = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
type ADL_Display_Color_Get_t = unsafe extern "C" fn(i32, i32, i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> i32;

const ADL_DISPLAY_COLOR_SATURATION: i32 = 9;

extern "C" fn adl_malloc(size: i32) -> *mut std::ffi::c_void {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(size as usize, 1)) as *mut _ }
}

struct AmdApi {
    color_set: ADL_Display_Color_Set_t,
    color_get: ADL_Display_Color_Get_t,
    adapter_count: i32,
}

impl AmdApi {
    fn load() -> Option<Self> {
        unsafe {
            let dll_name = if cfg!(target_pointer_width = "64") {
                b"atiadlxx.dll\0"
            } else {
                b"atiadlxy.dll\0"
            };
            let dll = LoadLibraryA(PCSTR(dll_name.as_ptr())).ok()?;

            let create_fn = GetProcAddress(dll, PCSTR(b"ADL_Main_Control_Create\0".as_ptr()))?;
            let create: ADL_Main_Control_Create_t = std::mem::transmute(create_fn);

            let num_fn = GetProcAddress(dll, PCSTR(b"ADL_Adapter_NumberOfAdapters_Get\0".as_ptr()))?;
            let num_adapters: ADL_Adapter_NumberOfAdapters_Get_t = std::mem::transmute(num_fn);

            let color_set_fn = GetProcAddress(dll, PCSTR(b"ADL_Display_Color_Set\0".as_ptr()))?;
            let color_set: ADL_Display_Color_Set_t = std::mem::transmute(color_set_fn);

            let color_get_fn = GetProcAddress(dll, PCSTR(b"ADL_Display_Color_Get\0".as_ptr()))?;
            let color_get: ADL_Display_Color_Get_t = std::mem::transmute(color_get_fn);

            if create(adl_malloc, 1) != 0 { return None; }

            let mut count: i32 = 0;
            num_adapters(&mut count);

            Some(AmdApi {
                color_set,
                color_get,
                adapter_count: count,
            })
        }
    }

    fn set_saturation(&self, level: i32) -> bool {
        unsafe {
            for adapter in 0..self.adapter_count {
                (self.color_set)(adapter, 0, ADL_DISPLAY_COLOR_SATURATION, level);
            }
            true
        }
    }

    fn get_saturation(&self) -> Option<i32> {
        unsafe {
            let mut cur = 0i32;
            let mut def = 0i32;
            let mut min = 0i32;
            let mut max = 0i32;
            let mut step = 0i32;
            if (self.color_get)(0, 0, ADL_DISPLAY_COLOR_SATURATION, &mut cur, &mut def, &mut min, &mut max, &mut step) == 0 {
                Some(cur)
            } else {
                None
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Unknown,
}

pub struct VibranceState {
    pub enabled: Arc<AtomicBool>,
    pub ingame_level: Arc<AtomicI32>,
    pub desktop_level: Arc<AtomicI32>,
    pub vendor: GpuVendor,
    running: Arc<AtomicBool>,
}

pub const NVIDIA_DEFAULT: i32 = 0;
pub const NVIDIA_MAX: i32 = 63;
pub const NVIDIA_MIN: i32 = 0;

pub const AMD_DEFAULT: i32 = 100;
pub const AMD_MAX: i32 = 200;
pub const AMD_MIN: i32 = 0;

impl VibranceState {
    pub fn new(is_amd: bool) -> Self {
        let vendor = if is_amd { GpuVendor::Amd } else { GpuVendor::Nvidia };
        let (default_ingame, default_desktop) = if is_amd {
            (AMD_MAX, AMD_DEFAULT)
        } else {
            (NVIDIA_MAX, NVIDIA_DEFAULT)
        };
        VibranceState {
            enabled: Arc::new(AtomicBool::new(false)),
            ingame_level: Arc::new(AtomicI32::new(default_ingame)),
            desktop_level: Arc::new(AtomicI32::new(default_desktop)),
            vendor,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn max_level(&self) -> i32 {
        match self.vendor {
            GpuVendor::Nvidia => NVIDIA_MAX,
            GpuVendor::Amd => AMD_MAX,
            GpuVendor::Unknown => 100,
        }
    }

    pub fn min_level(&self) -> i32 {
        match self.vendor {
            GpuVendor::Nvidia => NVIDIA_MIN,
            GpuVendor::Amd => AMD_MIN,
            GpuVendor::Unknown => 0,
        }
    }

    pub fn default_level(&self) -> i32 {
        match self.vendor {
            GpuVendor::Nvidia => NVIDIA_DEFAULT,
            GpuVendor::Amd => AMD_DEFAULT,
            GpuVendor::Unknown => 50,
        }
    }

    pub fn start(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);

        let enabled = self.enabled.clone();
        let ingame = self.ingame_level.clone();
        let desktop = self.desktop_level.clone();
        let running = self.running.clone();
        let vendor = self.vendor;

        thread::spawn(move || {
            run_vibrance_hook(vendor, enabled, ingame, desktop, running);
        });
    }

    pub fn apply_immediate(&self) {
        let level = self.ingame_level.load(Ordering::Relaxed);
        let vendor = self.vendor;
        thread::spawn(move || {
            if is_valorant_foreground() {
                match vendor {
                    GpuVendor::Nvidia => {
                        if let Some(api) = NvidiaApi::load() {
                            api.set_level(level);
                        }
                    }
                    GpuVendor::Amd => {
                        if let Some(api) = AmdApi::load() {
                            api.set_saturation(level);
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    pub fn stop(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }
}

const VALORANT_PROCESS: &str = "VALORANT-Win64-Shipping.exe";

fn is_valorant_foreground() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return false;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return false;
        }
        process_name_by_pid(pid)
            .map(|name| name.eq_ignore_ascii_case(VALORANT_PROCESS))
            .unwrap_or(false)
    }
}

fn process_name_by_pid(pid: u32) -> Option<String> {
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::Foundation::CloseHandle;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        use windows::Win32::System::Threading::QueryFullProcessImageNameW;
        use windows::Win32::System::Threading::PROCESS_NAME_FORMAT;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut size);
        let _ = CloseHandle(handle);
        if ok.is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

fn run_vibrance_hook(
    vendor: GpuVendor,
    enabled: Arc<AtomicBool>,
    ingame: Arc<AtomicI32>,
    desktop: Arc<AtomicI32>,
    running: Arc<AtomicBool>,
) {
    let set_level: Box<dyn Fn(i32) -> bool> = match vendor {
        GpuVendor::Nvidia => {
            if let Some(api) = NvidiaApi::load() {
                Box::new(move |level| api.set_level(level))
            } else {
                running.store(false, Ordering::Relaxed);
                return;
            }
        }
        GpuVendor::Amd => {
            if let Some(api) = AmdApi::load() {
                Box::new(move |level| api.set_saturation(level))
            } else {
                running.store(false, Ordering::Relaxed);
                return;
            }
        }
        GpuVendor::Unknown => {
            running.store(false, Ordering::Relaxed);
            return;
        }
    };

    let enabled_c = enabled.clone();
    let ingame_c = ingame.clone();
    let desktop_c = desktop.clone();

    let hook_callback = move || {
        if !enabled_c.load(Ordering::Relaxed) {
            return;
        }
        if is_valorant_foreground() {
            set_level(ingame_c.load(Ordering::Relaxed));
        } else {
            set_level(desktop_c.load(Ordering::Relaxed));
        }
    };

    unsafe {
        static mut HOOK_FN: Option<Box<dyn Fn()>> = None;
        HOOK_FN = Some(Box::new(hook_callback));

        unsafe extern "system" fn win_event_proc(
            _h: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
            _event: u32,
            _hwnd: HWND,
            _id_object: i32,
            _id_child: i32,
            _dw_event_thread: u32,
            _dwms_event_time: u32,
        ) {
            let ptr = &raw const HOOK_FN;
            if let Some(f) = &*ptr {
                f();
            }
        }

        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        if hook.is_invalid() {
            running.store(false, Ordering::Relaxed);
            return;
        }

        if let Some(f) = &*(&raw const HOOK_FN) {
            f();
        }

        let mut msg = MSG::default();
        while enabled.load(Ordering::Relaxed) {
            if GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
                DispatchMessageW(&msg);
            } else {
                break;
            }
        }

        let _ = UnhookWinEvent(hook);

        let final_desktop = desktop.load(Ordering::Relaxed);
        match vendor {
            GpuVendor::Nvidia => {
                if let Some(api) = NvidiaApi::load() {
                    api.set_level(final_desktop);
                }
            }
            GpuVendor::Amd => {
                if let Some(api) = AmdApi::load() {
                    api.set_saturation(final_desktop);
                }
            }
            _ => {}
        }
    }

    running.store(false, Ordering::Relaxed);
}
