use std::mem::size_of;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    SendInput, VIRTUAL_KEY,
};
use windows::core::{Error, HRESULT, Result};

use crate::action::{Action, KeyCode, ensure_python_runtime_available};
use crate::event::{HotkeyName, MouseButton, TouchEvent};

const INPUT_SETTLE_DELAY: Duration = Duration::from_millis(90);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
static INPUT_LOCK: Mutex<()> = Mutex::new(());

/// 파싱된 TouchBridge 이벤트를 Windows 입력으로 변환합니다.
pub fn send_event(event: &TouchEvent) -> Result<()> {
    let _guard = INPUT_LOCK.lock().expect("input lock poisoned");
    let result = send_event_inner(event);
    thread::sleep(INPUT_SETTLE_DELAY);
    result
}

pub fn send_action(action: &Action) -> Result<()> {
    let _guard = INPUT_LOCK.lock().expect("input lock poisoned");
    let result = send_action_inner(action);
    thread::sleep(INPUT_SETTLE_DELAY);
    result
}

fn send_event_inner(event: &TouchEvent) -> Result<()> {
    match event {
        TouchEvent::Move { dx, dy } => send_mouse_move(*dx, *dy),
        TouchEvent::Click { button } => send_mouse_click(button),
        TouchEvent::Scroll { dy } => send_mouse_scroll(*dy),
        TouchEvent::Hotkey { name } => send_hotkey(name),
        TouchEvent::Gesture { .. }
        | TouchEvent::GestureEvent { .. }
        | TouchEvent::Handshake { .. }
        | TouchEvent::CustomButtonSync { .. }
        | TouchEvent::CustomButtonEvent { .. } => Ok(()),
    }
}

fn send_action_inner(action: &Action) -> Result<()> {
    match action {
        Action::None => Ok(()),
        Action::Hotkey { keys } => send_hotkey_keys(keys),
        Action::PythonScript { code } => run_python_code(code),
        Action::PowerShellScript { script } => run_powershell_script(script),
    }
}

fn run_python_code(code: &str) -> Result<()> {
    let runtime = ensure_python_runtime_available().map_err(input_error)?;
    let mut command = Command::new(runtime.program());
    command.args(runtime.prefix_args()).arg("-c").arg(code);
    run_script_command(command, "Python")
}

fn run_powershell_script(script: &str) -> Result<()> {
    let mut command = Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script);
    run_script_command(command, "PowerShell")
}

fn run_script_command(mut command: Command, label: &str) -> Result<()> {
    let output = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| input_error(format!("{label} 실행 시작 실패: {err}")))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };

    if detail.is_empty() {
        Err(input_error(format!(
            "{label} 실행 실패: 종료 코드 {:?}",
            output.status.code()
        )))
    } else {
        Err(input_error(format!(
            "{label} 실행 실패: {}",
            truncate_error(&detail)
        )))
    }
}

fn truncate_error(value: &str) -> String {
    const LIMIT: usize = 600;

    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(LIMIT).collect();

    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn input_error(message: impl AsRef<str>) -> Error {
    Error::new(E_FAIL, message)
}

fn send_mouse_move(dx: i32, dy: i32) -> Result<()> {
    let input = mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE);
    send_inputs(&[input])
}

fn send_mouse_click(button: &MouseButton) -> Result<()> {
    let (down_flag, up_flag) = match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
    };

    let inputs = [
        mouse_input(0, 0, 0, down_flag),
        mouse_input(0, 0, 0, up_flag),
    ];

    send_inputs(&inputs)
}

fn send_mouse_scroll(dy: i32) -> Result<()> {
    // Windows의 wheel delta는 MOUSEINPUT.mouseData에 들어갑니다.
    // 양수는 위쪽, 음수는 아래쪽 스크롤입니다. Win32는 DWORD를 받으므로
    // 음수 i32 값을 2의 보수 u32 표현으로 전달합니다.
    let input = mouse_input(0, 0, dy as u32, MOUSEEVENTF_WHEEL);
    send_inputs(&[input])
}

fn send_hotkey(name: &HotkeyName) -> Result<()> {
    match name {
        HotkeyName::TaskManager => send_task_manager_hotkey(),
    }
}

fn send_task_manager_hotkey() -> Result<()> {
    send_hotkey_keys(&[KeyCode::Ctrl, KeyCode::Shift, KeyCode::Esc])
}

fn send_hotkey_keys(keys: &[KeyCode]) -> Result<()> {
    // Windows 셸 단축키는 가상키 일괄 전송보다 스캔코드와 짧은 hold가 더 안정적입니다.
    // 특히 Alt+Tab, Win+Tab, Alt+Left 같은 조합에서 입력 누락을 줄입니다.
    for key in keys {
        send_inputs(&[keyboard_input(*key, false)])?;
        thread::sleep(Duration::from_millis(12));
    }

    thread::sleep(Duration::from_millis(55));

    for key in keys.iter().rev() {
        send_inputs(&[keyboard_input(*key, true)])?;
        thread::sleep(Duration::from_millis(8));
    }

    Ok(())
}

fn scan_code(key: KeyCode) -> (u16, bool) {
    match key {
        KeyCode::Ctrl => (0x1D, false),
        KeyCode::Shift => (0x2A, false),
        KeyCode::Alt => (0x38, false),
        KeyCode::Win => (0x5B, true),
        KeyCode::Esc => (0x01, false),
        KeyCode::Enter => (0x1C, false),
        KeyCode::Tab => (0x0F, false),
        KeyCode::Space => (0x39, false),
        KeyCode::Backspace => (0x0E, false),
        KeyCode::Delete => (0x53, true),
        KeyCode::Left => (0x4B, true),
        KeyCode::Right => (0x4D, true),
        KeyCode::Up => (0x48, true),
        KeyCode::Down => (0x50, true),
        KeyCode::A => (0x1E, false),
        KeyCode::B => (0x30, false),
        KeyCode::C => (0x2E, false),
        KeyCode::D => (0x20, false),
        KeyCode::E => (0x12, false),
        KeyCode::F => (0x21, false),
        KeyCode::G => (0x22, false),
        KeyCode::H => (0x23, false),
        KeyCode::I => (0x17, false),
        KeyCode::J => (0x24, false),
        KeyCode::K => (0x25, false),
        KeyCode::L => (0x26, false),
        KeyCode::M => (0x32, false),
        KeyCode::N => (0x31, false),
        KeyCode::O => (0x18, false),
        KeyCode::P => (0x19, false),
        KeyCode::Q => (0x10, false),
        KeyCode::R => (0x13, false),
        KeyCode::S => (0x1F, false),
        KeyCode::T => (0x14, false),
        KeyCode::U => (0x16, false),
        KeyCode::V => (0x2F, false),
        KeyCode::W => (0x11, false),
        KeyCode::X => (0x2D, false),
        KeyCode::Y => (0x15, false),
        KeyCode::Z => (0x2C, false),
        KeyCode::Num0 => (0x0B, false),
        KeyCode::Num1 => (0x02, false),
        KeyCode::Num2 => (0x03, false),
        KeyCode::Num3 => (0x04, false),
        KeyCode::Num4 => (0x05, false),
        KeyCode::Num5 => (0x06, false),
        KeyCode::Num6 => (0x07, false),
        KeyCode::Num7 => (0x08, false),
        KeyCode::Num8 => (0x09, false),
        KeyCode::Num9 => (0x0A, false),
        KeyCode::F1 => (0x3B, false),
        KeyCode::F2 => (0x3C, false),
        KeyCode::F3 => (0x3D, false),
        KeyCode::F4 => (0x3E, false),
        KeyCode::F5 => (0x3F, false),
        KeyCode::F6 => (0x40, false),
        KeyCode::F7 => (0x41, false),
        KeyCode::F8 => (0x42, false),
        KeyCode::F9 => (0x43, false),
        KeyCode::F10 => (0x44, false),
        KeyCode::F11 => (0x57, false),
        KeyCode::F12 => (0x58, false),
    }
}

fn mouse_input(
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: windows::Win32::UI::Input::KeyboardAndMouse::MOUSE_EVENT_FLAGS,
) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: mouse_data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn keyboard_input(key: KeyCode, key_up: bool) -> INPUT {
    let (scan_code, extended) = scan_code(key);
    let mut flags = KEYEVENTF_SCANCODE;

    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }

    if key_up {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send_inputs(inputs: &[INPUT]) -> Result<()> {
    // SendInput은 Win32 API라 Rust 컴파일러가 메모리 안전성을 검증할 수 없습니다.
    // unsafe 범위를 이 함수 하나로 제한하고, 호출자는 안전한 Rust 함수만 사용하게 합니다.
    let sent = unsafe { SendInput(inputs, size_of::<INPUT>() as i32) };

    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(Error::from_thread())
    }
}
