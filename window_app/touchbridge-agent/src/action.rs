use serde::{Deserialize, Serialize};

use crate::i18n::{self, AppLanguage, TextKey};

use std::process::{Command, Stdio};

/// 제스처와 커스텀 버튼이 최종적으로 실행할 동작입니다.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    None,
    Hotkey {
        keys: Vec<KeyCode>,
    },
    PythonScript {
        code: String,
    },
    #[serde(rename = "powershell_script")]
    PowerShellScript {
        script: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    None,
    Hotkey,
    PythonScript,
    PowerShellScript,
}

impl ActionKind {
    pub const ALL: [Self; 4] = [
        Self::None,
        Self::Hotkey,
        Self::PythonScript,
        Self::PowerShellScript,
    ];

    pub fn label_for(self, language: AppLanguage) -> &'static str {
        match self {
            ActionKind::None => i18n::text(language, TextKey::NoAction),
            ActionKind::Hotkey => i18n::text(language, TextKey::HotkeyAction),
            ActionKind::PythonScript => i18n::text(language, TextKey::PythonScriptAction),
            ActionKind::PowerShellScript => i18n::text(language, TextKey::PowerShellScriptAction),
        }
    }
}

impl Action {
    pub fn hotkey(keys: Vec<KeyCode>) -> Self {
        Self::Hotkey { keys }
    }

    pub fn kind(&self) -> ActionKind {
        match self {
            Action::None => ActionKind::None,
            Action::Hotkey { .. } => ActionKind::Hotkey,
            Action::PythonScript { .. } => ActionKind::PythonScript,
            Action::PowerShellScript { .. } => ActionKind::PowerShellScript,
        }
    }

    pub fn display_text(&self) -> String {
        match self {
            Action::None => String::new(),
            Action::Hotkey { keys } => keys
                .iter()
                .map(|key| key.label())
                .collect::<Vec<_>>()
                .join("+"),
            Action::PythonScript { code } => code.clone(),
            Action::PowerShellScript { script } => script.clone(),
        }
    }

    pub fn summary(&self) -> String {
        self.summary_for(AppLanguage::English)
    }

    pub fn summary_for(&self, language: AppLanguage) -> String {
        match self {
            Action::None => i18n::text(language, TextKey::NoAction).to_string(),
            Action::Hotkey { keys } => format!(
                "{} {}",
                i18n::text(language, TextKey::HotkeyAction),
                keys_to_text(keys)
            ),
            Action::PythonScript { .. } => {
                i18n::text(language, TextKey::PythonScriptAction).to_string()
            }
            Action::PowerShellScript { .. } => {
                i18n::text(language, TextKey::PowerShellScriptAction).to_string()
            }
        }
    }
}

pub fn parse_action(kind: ActionKind, value: &str) -> Result<Action, String> {
    match kind {
        ActionKind::None => Ok(Action::None),
        ActionKind::Hotkey => parse_hotkey(value),
        ActionKind::PythonScript => parse_python_script(value),
        ActionKind::PowerShellScript => parse_powershell_script(value),
    }
}

pub fn parse_hotkey(value: &str) -> Result<Action, String> {
    let value = value.trim();

    if value.is_empty() || value.eq_ignore_ascii_case("none") || value == "-" {
        return Ok(Action::None);
    }

    let mut keys = Vec::new();

    for raw_part in value.split('+') {
        let part = raw_part.trim();

        if part.is_empty() {
            return Err("empty key segment".to_string());
        }

        let key = KeyCode::from_token(part).ok_or_else(|| format!("unsupported key: {part}"))?;

        if keys.contains(&key) {
            return Err(format!("duplicate key: {}", key.label()));
        }

        keys.push(key);
    }

    Ok(Action::Hotkey { keys })
}

fn parse_python_script(value: &str) -> Result<Action, String> {
    ensure_python_runtime_available()?;

    if value.trim().is_empty() {
        return Err("python code is empty".to_string());
    }

    Ok(Action::PythonScript {
        code: value.to_string(),
    })
}

fn parse_powershell_script(value: &str) -> Result<Action, String> {
    if value.trim().is_empty() {
        return Err("powershell script is empty".to_string());
    }

    Ok(Action::PowerShellScript {
        script: value.to_string(),
    })
}

pub fn keys_to_text(keys: &[KeyCode]) -> String {
    keys.iter()
        .map(|key| key.label())
        .collect::<Vec<_>>()
        .join("+")
}

/// GUI와 설정 파일에서 사용하는 키 이름입니다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCode {
    Ctrl,
    Shift,
    Alt,
    Win,
    Esc,
    Enter,
    Tab,
    Space,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl KeyCode {
    pub fn label(self) -> &'static str {
        match self {
            KeyCode::Ctrl => "Ctrl",
            KeyCode::Shift => "Shift",
            KeyCode::Alt => "Alt",
            KeyCode::Win => "Win",
            KeyCode::Esc => "Esc",
            KeyCode::Enter => "Enter",
            KeyCode::Tab => "Tab",
            KeyCode::Space => "Space",
            KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::Left => "Left",
            KeyCode::Right => "Right",
            KeyCode::Up => "Up",
            KeyCode::Down => "Down",
            KeyCode::A => "A",
            KeyCode::B => "B",
            KeyCode::C => "C",
            KeyCode::D => "D",
            KeyCode::E => "E",
            KeyCode::F => "F",
            KeyCode::G => "G",
            KeyCode::H => "H",
            KeyCode::I => "I",
            KeyCode::J => "J",
            KeyCode::K => "K",
            KeyCode::L => "L",
            KeyCode::M => "M",
            KeyCode::N => "N",
            KeyCode::O => "O",
            KeyCode::P => "P",
            KeyCode::Q => "Q",
            KeyCode::R => "R",
            KeyCode::S => "S",
            KeyCode::T => "T",
            KeyCode::U => "U",
            KeyCode::V => "V",
            KeyCode::W => "W",
            KeyCode::X => "X",
            KeyCode::Y => "Y",
            KeyCode::Z => "Z",
            KeyCode::Num0 => "0",
            KeyCode::Num1 => "1",
            KeyCode::Num2 => "2",
            KeyCode::Num3 => "3",
            KeyCode::Num4 => "4",
            KeyCode::Num5 => "5",
            KeyCode::Num6 => "6",
            KeyCode::Num7 => "7",
            KeyCode::Num8 => "8",
            KeyCode::Num9 => "9",
            KeyCode::F1 => "F1",
            KeyCode::F2 => "F2",
            KeyCode::F3 => "F3",
            KeyCode::F4 => "F4",
            KeyCode::F5 => "F5",
            KeyCode::F6 => "F6",
            KeyCode::F7 => "F7",
            KeyCode::F8 => "F8",
            KeyCode::F9 => "F9",
            KeyCode::F10 => "F10",
            KeyCode::F11 => "F11",
            KeyCode::F12 => "F12",
        }
    }

    pub fn from_token(value: &str) -> Option<Self> {
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '_', '-'], "");

        match normalized.as_str() {
            "ctrl" | "control" => Some(KeyCode::Ctrl),
            "shift" => Some(KeyCode::Shift),
            "alt" | "menu" => Some(KeyCode::Alt),
            "win" | "windows" | "meta" => Some(KeyCode::Win),
            "esc" | "escape" => Some(KeyCode::Esc),
            "enter" | "return" => Some(KeyCode::Enter),
            "tab" => Some(KeyCode::Tab),
            "space" => Some(KeyCode::Space),
            "backspace" | "back" => Some(KeyCode::Backspace),
            "delete" | "del" => Some(KeyCode::Delete),
            "insert" | "ins" => Some(KeyCode::Insert),
            "home" => Some(KeyCode::Home),
            "end" => Some(KeyCode::End),
            "pageup" | "pgup" => Some(KeyCode::PageUp),
            "pagedown" | "pgdn" => Some(KeyCode::PageDown),
            "left" | "arrowleft" => Some(KeyCode::Left),
            "right" | "arrowright" => Some(KeyCode::Right),
            "up" | "arrowup" => Some(KeyCode::Up),
            "down" | "arrowdown" => Some(KeyCode::Down),
            "keya" => Some(KeyCode::A),
            "keyb" => Some(KeyCode::B),
            "keyc" => Some(KeyCode::C),
            "keyd" => Some(KeyCode::D),
            "keye" => Some(KeyCode::E),
            "keyf" => Some(KeyCode::F),
            "keyg" => Some(KeyCode::G),
            "keyh" => Some(KeyCode::H),
            "keyi" => Some(KeyCode::I),
            "keyj" => Some(KeyCode::J),
            "keyk" => Some(KeyCode::K),
            "keyl" => Some(KeyCode::L),
            "keym" => Some(KeyCode::M),
            "keyn" => Some(KeyCode::N),
            "keyo" => Some(KeyCode::O),
            "keyp" => Some(KeyCode::P),
            "keyq" => Some(KeyCode::Q),
            "keyr" => Some(KeyCode::R),
            "keys" => Some(KeyCode::S),
            "keyt" => Some(KeyCode::T),
            "keyu" => Some(KeyCode::U),
            "keyv" => Some(KeyCode::V),
            "keyw" => Some(KeyCode::W),
            "keyx" => Some(KeyCode::X),
            "keyy" => Some(KeyCode::Y),
            "keyz" => Some(KeyCode::Z),
            "a" => Some(KeyCode::A),
            "b" => Some(KeyCode::B),
            "c" => Some(KeyCode::C),
            "d" => Some(KeyCode::D),
            "e" => Some(KeyCode::E),
            "f" => Some(KeyCode::F),
            "g" => Some(KeyCode::G),
            "h" => Some(KeyCode::H),
            "i" => Some(KeyCode::I),
            "j" => Some(KeyCode::J),
            "k" => Some(KeyCode::K),
            "l" => Some(KeyCode::L),
            "m" => Some(KeyCode::M),
            "n" => Some(KeyCode::N),
            "o" => Some(KeyCode::O),
            "p" => Some(KeyCode::P),
            "q" => Some(KeyCode::Q),
            "r" => Some(KeyCode::R),
            "s" => Some(KeyCode::S),
            "t" => Some(KeyCode::T),
            "u" => Some(KeyCode::U),
            "v" => Some(KeyCode::V),
            "w" => Some(KeyCode::W),
            "x" => Some(KeyCode::X),
            "y" => Some(KeyCode::Y),
            "z" => Some(KeyCode::Z),
            "0" | "num0" => Some(KeyCode::Num0),
            "1" | "num1" => Some(KeyCode::Num1),
            "2" | "num2" => Some(KeyCode::Num2),
            "3" | "num3" => Some(KeyCode::Num3),
            "4" | "num4" => Some(KeyCode::Num4),
            "5" | "num5" => Some(KeyCode::Num5),
            "6" | "num6" => Some(KeyCode::Num6),
            "7" | "num7" => Some(KeyCode::Num7),
            "8" | "num8" => Some(KeyCode::Num8),
            "9" | "num9" => Some(KeyCode::Num9),
            "f1" => Some(KeyCode::F1),
            "f2" => Some(KeyCode::F2),
            "f3" => Some(KeyCode::F3),
            "f4" => Some(KeyCode::F4),
            "f5" => Some(KeyCode::F5),
            "f6" => Some(KeyCode::F6),
            "f7" => Some(KeyCode::F7),
            "f8" => Some(KeyCode::F8),
            "f9" => Some(KeyCode::F9),
            "f10" => Some(KeyCode::F10),
            "f11" => Some(KeyCode::F11),
            "f12" => Some(KeyCode::F12),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PythonRuntime {
    PyLauncher,
    Python,
    Python3,
}

impl PythonRuntime {
    pub fn program(self) -> &'static str {
        match self {
            PythonRuntime::PyLauncher => "py",
            PythonRuntime::Python => "python",
            PythonRuntime::Python3 => "python3",
        }
    }

    pub fn prefix_args(self) -> &'static [&'static str] {
        match self {
            PythonRuntime::PyLauncher => &["-3"],
            PythonRuntime::Python | PythonRuntime::Python3 => &[],
        }
    }
}

pub fn ensure_python_runtime_available() -> Result<PythonRuntime, String> {
    detect_python_runtime().ok_or_else(|| "python runtime was not found".to_string())
}

pub fn detect_python_runtime() -> Option<PythonRuntime> {
    [
        PythonRuntime::PyLauncher,
        PythonRuntime::Python,
        PythonRuntime::Python3,
    ]
    .into_iter()
    .find(|runtime| python_runtime_responds(*runtime))
}

fn python_runtime_responds(runtime: PythonRuntime) -> bool {
    Command::new(runtime.program())
        .args(runtime.prefix_args())
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
