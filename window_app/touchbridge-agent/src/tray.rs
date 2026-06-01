use std::mem::size_of;
use std::process;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NIN_SELECT,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, FindWindowExW, FindWindowW, GetCursorPos, GetMessageW, HWND_MESSAGE,
    IDI_APPLICATION, LoadIconW, MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    SW_RESTORE, SetForegroundWindow, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TRACK_POPUP_MENU_FLAGS, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_APP, WM_CLOSE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
    WM_RBUTTONUP, WM_USER, WNDCLASSW,
};
use windows::core::{Error, PCWSTR, w};

use crate::i18n::{self, AppLanguage, TextKey};

const TRAY_ICON_ID: u32 = 1;
const WM_TRAY_ICON: u32 = WM_USER + 1;
const WM_SHOW_EXISTING_INSTANCE: u32 = WM_APP + 1;
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;
const MENU_SHOW_ID: usize = 1001;
const MENU_EXIT_ID: usize = 1002;

#[derive(Clone)]
pub struct TrayHandle {
    signals: Arc<TraySignals>,
}

struct TraySignals {
    available: AtomicBool,
    language: AtomicU8,
    show_requested: AtomicBool,
    exit_requested: AtomicBool,
    force_exit_started: AtomicBool,
    repaint_context: Mutex<Option<egui::Context>>,
}

static TRAY_SIGNALS: OnceLock<Arc<TraySignals>> = OnceLock::new();

impl TrayHandle {
    pub fn start(language: AppLanguage) -> Self {
        let signals = Arc::new(TraySignals {
            available: AtomicBool::new(false),
            language: AtomicU8::new(language_to_u8(language)),
            show_requested: AtomicBool::new(false),
            exit_requested: AtomicBool::new(false),
            force_exit_started: AtomicBool::new(false),
            repaint_context: Mutex::new(None),
        });

        let _ = TRAY_SIGNALS.set(signals.clone());

        thread::spawn(|| {
            if let Err(err) = run_tray_loop() {
                eprintln!("Tray service stopped: {err}");
            }
        });

        Self { signals }
    }

    pub fn take_show_requested(&self) -> bool {
        self.signals.show_requested.swap(false, Ordering::SeqCst)
    }

    pub fn take_exit_requested(&self) -> bool {
        self.signals.exit_requested.swap(false, Ordering::SeqCst)
    }

    pub fn set_language(&self, language: AppLanguage) {
        self.signals
            .language
            .store(language_to_u8(language), Ordering::SeqCst);
    }

    pub fn set_repaint_context(&self, context: egui::Context) {
        let Ok(mut repaint_context) = self.signals.repaint_context.lock() else {
            return;
        };

        *repaint_context = Some(context);
    }

    pub fn request_exit(&self) {
        request_exit();
    }
}

pub fn request_existing_instance_show() -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);

    while Instant::now() < deadline {
        if let Ok(hwnd) = unsafe {
            FindWindowExW(
                Some(HWND_MESSAGE),
                None,
                w!("TouchBridgeAgentTrayWindow"),
                w!("TouchBridge Agent Tray"),
            )
        } {
            if unsafe { PostMessageW(Some(hwnd), WM_SHOW_EXISTING_INSTANCE, WPARAM(0), LPARAM(0)) }
                .is_ok()
            {
                return true;
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    false
}

fn run_tray_loop() -> windows::core::Result<()> {
    let class_name = w!("TouchBridgeAgentTrayWindow");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(tray_window_proc),
        lpszClassName: class_name,
        ..Default::default()
    };

    unsafe {
        RegisterClassW(&window_class);
    }

    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("TouchBridge Agent Tray"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        )?
    };

    add_tray_icon(hwnd)?;
    set_available(true);

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0).as_bool() } {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe {
        delete_tray_icon(hwnd);
        let _ = DestroyWindow(hwnd);
    }
    set_available(false);

    Ok(())
}

fn add_tray_icon(hwnd: HWND) -> windows::core::Result<()> {
    let mut data = base_notify_icon_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    data.hIcon = unsafe { LoadIconW(None, IDI_APPLICATION)? };
    data.szTip = wide_tip("TouchBridge Agent");

    unsafe {
        if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
            return Err(Error::from_thread());
        }

        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
    }

    Ok(())
}

unsafe fn delete_tray_icon(hwnd: HWND) {
    let data = base_notify_icon_data(hwnd);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn base_notify_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    }
}

fn wide_tip(value: &str) -> [u16; 128] {
    let mut buffer = [0u16; 128];

    for (index, unit) in value.encode_utf16().take(buffer.len() - 1).enumerate() {
        buffer[index] = unit;
    }

    buffer
}

fn wide_menu_text(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn language_to_u8(language: AppLanguage) -> u8 {
    match language {
        AppLanguage::English => 0,
        AppLanguage::Korean => 1,
    }
}

fn current_language() -> AppLanguage {
    let Some(signals) = TRAY_SIGNALS.get() else {
        return AppLanguage::English;
    };

    match signals.language.load(Ordering::SeqCst) {
        1 => AppLanguage::Korean,
        _ => AppLanguage::English,
    }
}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SHOW_EXISTING_INSTANCE => {
            request_show();
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            if let Some(event) = tray_event(wparam, lparam) {
                match event {
                    WM_LBUTTONUP | WM_LBUTTONDBLCLK | NIN_SELECT | NIN_KEYSELECT => request_show(),
                    WM_RBUTTONUP | WM_CONTEXTMENU => unsafe {
                        show_context_menu(hwnd);
                    },
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            match wparam.0 & 0xffff {
                MENU_SHOW_ID => request_show(),
                MENU_EXIT_ID => request_exit(),
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn tray_event(wparam: WPARAM, lparam: LPARAM) -> Option<u32> {
    if wparam.0 as u32 == TRAY_ICON_ID {
        return Some(lparam.0 as u32);
    }

    let lparam_value = lparam.0 as usize;
    let event = (lparam_value & 0xffff) as u32;
    let icon_id = ((lparam_value >> 16) & 0xffff) as u32;

    (icon_id == TRAY_ICON_ID).then_some(event)
}

fn request_show() {
    if let Some(signals) = TRAY_SIGNALS.get() {
        signals.show_requested.store(true, Ordering::SeqCst);
        request_gui_repaint(signals);
    }

    show_main_window();
}

fn set_available(available: bool) {
    if let Some(signals) = TRAY_SIGNALS.get() {
        signals.available.store(available, Ordering::SeqCst);
    }
}

fn request_exit() {
    if let Some(signals) = TRAY_SIGNALS.get() {
        signals.exit_requested.store(true, Ordering::SeqCst);
        request_gui_repaint(signals);
        schedule_force_exit(signals);
    }

    close_main_window();
    close_tray_window();
}

fn schedule_force_exit(signals: &TraySignals) {
    if signals.force_exit_started.swap(true, Ordering::SeqCst) {
        return;
    }

    thread::spawn(|| {
        thread::sleep(Duration::from_millis(1200));
        process::exit(0);
    });
}

fn request_gui_repaint(signals: &TraySignals) {
    let Ok(repaint_context) = signals.repaint_context.lock() else {
        return;
    };

    if let Some(context) = repaint_context.as_ref() {
        context.request_repaint();
    }
}

fn show_main_window() {
    let Some(hwnd) = main_window() else {
        return;
    };

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn close_main_window() {
    let Some(hwnd) = main_window() else {
        return;
    };

    let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

fn close_tray_window() {
    let Ok(hwnd) = (unsafe {
        FindWindowExW(
            Some(HWND_MESSAGE),
            None,
            w!("TouchBridgeAgentTrayWindow"),
            w!("TouchBridge Agent Tray"),
        )
    }) else {
        return;
    };

    let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

fn main_window() -> Option<HWND> {
    unsafe { FindWindowW(PCWSTR::null(), w!("TouchBridge Agent")) }.ok()
}

unsafe fn show_context_menu(hwnd: HWND) {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };

    unsafe {
        let language = current_language();
        let open_text = wide_menu_text(i18n::text(language, TextKey::OpenTouchBridge));
        let exit_text = wide_menu_text(i18n::text(language, TextKey::Exit));

        let _ = AppendMenuW(menu, MF_STRING, MENU_SHOW_ID, PCWSTR(open_text.as_ptr()));
        let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT_ID, PCWSTR(exit_text.as_ptr()));
    }

    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point).is_err() } {
        point.x = 0;
        point.y = 0;
    }

    unsafe {
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD | TRACK_POPUP_MENU_FLAGS(0),
            point.x,
            point.y,
            None,
            hwnd,
            None,
        );

        match command.0 as usize {
            MENU_SHOW_ID => request_show(),
            MENU_EXIT_ID => request_exit(),
            _ => {}
        }

        let _ = DestroyMenu(menu);
    }
}
