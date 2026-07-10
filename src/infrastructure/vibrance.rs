#![allow(non_camel_case_types)]

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

use windows::core::PCSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    PostThreadMessageW, TranslateMessage, EVENT_SYSTEM_FOREGROUND, MSG, WINEVENT_OUTOFCONTEXT,
    WM_QUIT, WM_USER,
};

type NvAPI_QueryInterface_t = unsafe extern "C" fn(u32) -> *const ();
type NvAPI_Initialize_t = unsafe extern "C" fn() -> i32;
type NvAPI_EnumNvidiaDisplayHandle_t = unsafe extern "C" fn(i32, *mut i32) -> i32;
type NvAPI_SetDVCLevel_t = unsafe extern "C" fn(i32, i32, i32) -> i32;
const NVAPI_INITIALIZE_ID: u32 = 0x0150E828;
const NVAPI_ENUM_DISPLAY_HANDLE_ID: u32 = 0x9ABDD40D;
const NVAPI_SET_DVC_LEVEL_ID: u32 = 0x172409B4;
struct NvidiaApi {
    set_dvc: NvAPI_SetDVCLevel_t,
    display_handle: i32,
}

impl NvidiaApi {
    fn load() -> Option<Self> {
        unsafe {
            let dll = LoadLibraryA(PCSTR::from_raw(c"nvapi64.dll".as_ptr().cast())).ok()?;
            let query_fn = GetProcAddress(
                dll,
                PCSTR::from_raw(c"nvapi_QueryInterface".as_ptr().cast()),
            )?;
            let query: NvAPI_QueryInterface_t = std::mem::transmute(query_fn);

            let init_ptr = query(NVAPI_INITIALIZE_ID);
            if init_ptr.is_null() {
                return None;
            }
            let initialize: NvAPI_Initialize_t = std::mem::transmute(init_ptr);

            let enum_ptr = query(NVAPI_ENUM_DISPLAY_HANDLE_ID);
            if enum_ptr.is_null() {
                return None;
            }
            let enum_display: NvAPI_EnumNvidiaDisplayHandle_t = std::mem::transmute(enum_ptr);

            let set_ptr = query(NVAPI_SET_DVC_LEVEL_ID);
            if set_ptr.is_null() {
                return None;
            }
            let set_dvc: NvAPI_SetDVCLevel_t = std::mem::transmute(set_ptr);

            if initialize() != 0 {
                return None;
            }

            let mut handle: i32 = 0;
            if enum_display(0, &mut handle) != 0 {
                return None;
            }

            Some(NvidiaApi {
                set_dvc,
                display_handle: handle,
            })
        }
    }

    fn set_level(&self, level: i32) -> bool {
        unsafe { (self.set_dvc)(self.display_handle, 0, level) == 0 }
    }
}

pub struct VibranceState {
    pub enabled: Arc<AtomicBool>,
    pub ingame_level: Arc<AtomicI32>,
    pub desktop_level: Arc<AtomicI32>,
    running: Arc<AtomicBool>,
    hook_thread_id: Arc<AtomicU32>,
}

pub const NVIDIA_DEFAULT: i32 = 0;
pub const NVIDIA_MAX: i32 = 63;
impl VibranceState {
    pub fn new() -> Self {
        VibranceState {
            enabled: Arc::new(AtomicBool::new(false)),
            ingame_level: Arc::new(AtomicI32::new(NVIDIA_MAX)),
            desktop_level: Arc::new(AtomicI32::new(NVIDIA_DEFAULT)),
            running: Arc::new(AtomicBool::new(false)),
            hook_thread_id: Arc::new(AtomicU32::new(0)),
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
        let hook_thread_id = self.hook_thread_id.clone();

        thread::spawn(move || {
            run_vibrance_hook(enabled, ingame, desktop, running, hook_thread_id);
        });
    }

    pub fn apply_immediate(&self) {
        let tid = self.hook_thread_id.load(Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_USER, None, None);
            }
        }
    }

    pub fn stop(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        let tid = self.hook_thread_id.load(Ordering::Relaxed);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, None, None);
            }
        }
    }
}

const VALORANT_PROCESS: &str = "VALORANT-Win64-Shipping.exe";

fn process_name_by_pid(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut size = buf.len() as u32;
        use windows::Win32::System::Threading::QueryFullProcessImageNameW;
        use windows::Win32::System::Threading::PROCESS_NAME_FORMAT;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if ok.is_err() {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit('\\').next().map(|s| s.to_string())
    }
}

struct HookState {
    ingame: Arc<AtomicI32>,
    desktop: Arc<AtomicI32>,
    set_level: Box<dyn Fn(i32) -> bool>,
    last_was_valorant: bool,
    valorant_pid: u32,
}

thread_local! {
    static HOOK_STATE: RefCell<Option<HookState>> = const { RefCell::new(None) };
}

fn is_valorant_window(hwnd: HWND) -> bool {
    unsafe {
        if hwnd.0.is_null() {
            return false;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return false;
        }
        HOOK_STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            if let Some(s) = state.as_mut() {
                if s.valorant_pid != 0 && s.valorant_pid == pid {
                    return true;
                }
                if let Some(name) = process_name_by_pid(pid) {
                    if name.eq_ignore_ascii_case(VALORANT_PROCESS) {
                        s.valorant_pid = pid;
                        return true;
                    }
                }
                if s.valorant_pid != 0 && process_name_by_pid(s.valorant_pid).is_none() {
                    s.valorant_pid = 0;
                }
                false
            } else {
                false
            }
        })
    }
}

fn apply_vibrance_for_foreground(hwnd: Option<HWND>) {
    let hwnd = hwnd.unwrap_or_else(|| unsafe { GetForegroundWindow() });
    let is_val = is_valorant_window(hwnd);
    HOOK_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if let Some(s) = state.as_mut() {
            if is_val != s.last_was_valorant {
                s.last_was_valorant = is_val;
                if is_val {
                    (s.set_level)(s.ingame.load(Ordering::Relaxed));
                } else {
                    (s.set_level)(s.desktop.load(Ordering::Relaxed));
                }
            }
        }
    });
}

fn force_apply_vibrance() {
    let hwnd = unsafe { GetForegroundWindow() };
    let is_val = is_valorant_window(hwnd);
    HOOK_STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        if let Some(s) = state.as_mut() {
            s.last_was_valorant = is_val;
            if is_val {
                (s.set_level)(s.ingame.load(Ordering::Relaxed));
            } else {
                (s.set_level)(s.desktop.load(Ordering::Relaxed));
            }
        }
    });
}

unsafe extern "system" fn win_event_proc(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _event_time: u32,
) {
    apply_vibrance_for_foreground(Some(hwnd));
}

fn run_vibrance_hook(
    enabled: Arc<AtomicBool>,
    ingame: Arc<AtomicI32>,
    desktop: Arc<AtomicI32>,
    running: Arc<AtomicBool>,
    hook_thread_id: Arc<AtomicU32>,
) {
    let Some(api) = NvidiaApi::load() else {
        running.store(false, Ordering::Relaxed);
        return;
    };
    let set_level: Box<dyn Fn(i32) -> bool> = Box::new(move |level| api.set_level(level));

    HOOK_STATE.with(|cell| {
        *cell.borrow_mut() = Some(HookState {
            ingame: ingame.clone(),
            desktop: desktop.clone(),
            set_level,
            last_was_valorant: false,
            valorant_pid: 0,
        });
    });

    unsafe {
        let tid = GetCurrentThreadId();
        hook_thread_id.store(tid, Ordering::Relaxed);

        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        apply_vibrance_for_foreground(None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
            if !enabled.load(Ordering::Relaxed) {
                break;
            }
            if msg.message == WM_USER {
                force_apply_vibrance();
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !hook.is_invalid() {
            let _ = UnhookWinEvent(hook);
        }

        hook_thread_id.store(0, Ordering::Relaxed);
    }

    HOOK_STATE.with(|cell| {
        if let Some(s) = cell.borrow().as_ref() {
            (s.set_level)(s.desktop.load(Ordering::Relaxed));
        }
        *cell.borrow_mut() = None;
    });

    running.store(false, Ordering::Relaxed);
}
