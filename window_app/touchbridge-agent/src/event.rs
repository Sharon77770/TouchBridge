use serde::{Deserialize, Serialize};

use crate::action::{KeyCode, keys_to_text};
use crate::i18n::AppLanguage;

#[derive(Debug)]
pub enum TouchEvent {
    Move {
        dx: i32,
        dy: i32,
    },
    Click {
        button: MouseButton,
    },
    Scroll {
        dy: i32,
    },
    MouseDelta {
        device_id: Option<String>,
        dx: i32,
        dy: i32,
        dt: u32,
        seq: u64,
        timestamp: Option<u64>,
    },
    KeyboardText {
        device_id: Option<String>,
        text: String,
        seq: Option<u64>,
        timestamp: Option<u64>,
    },
    KeyboardKey {
        device_id: Option<String>,
        key: KeyCode,
        modifiers: Vec<KeyCode>,
        seq: Option<u64>,
        timestamp: Option<u64>,
    },
    GestureEvent {
        device_id: Option<String>,
        gesture: Gesture,
        profile_id: Option<String>,
        timestamp: Option<u64>,
    },
    Handshake {
        device_id: String,
        client: Option<String>,
        protocol_version: Option<u32>,
        timestamp: Option<u64>,
    },
    CustomButtonSyncBegin {
        device_id: Option<String>,
    },
    CustomButtonSyncItem {
        device_id: Option<String>,
        button: CustomButtonDefinition,
    },
    CustomButtonSyncEnd {
        device_id: Option<String>,
    },
    CustomButtonEvent {
        device_id: Option<String>,
        button_id: String,
        timestamp: Option<u64>,
    },
    MouseButtonEvent {
        device_id: Option<String>,
        button: MouseButton,
        action: MouseButtonAction,
        timestamp: Option<u64>,
    },
}

#[derive(Debug)]
pub enum MouseButton {
    Left,
    Right,
}

#[derive(Debug)]
pub enum MouseButtonAction {
    Down,
    Up,
}

#[derive(Clone, Debug)]
pub struct CustomButtonDefinition {
    pub id: String,
    pub label: String,
    pub position: Option<usize>,
}

pub fn parse_compact_event(raw: &str) -> std::result::Result<TouchEvent, String> {
    let fields = raw.trim().split(':').collect::<Vec<_>>();
    let command = fields.first().copied().unwrap_or_default();

    match command {
        "H" => {
            let device_id = decode_compact_field(require_field(&fields, 1, "device id")?)?;
            Ok(TouchEvent::Handshake {
                device_id,
                client: Some("android".to_string()),
                protocol_version: Some(2),
                timestamp: None,
            })
        }
        "G" => Ok(TouchEvent::GestureEvent {
            device_id: None,
            gesture: parse_gesture(require_field(&fields, 1, "gesture")?)?,
            profile_id: None,
            timestamp: None,
        }),
        "M" => Ok(TouchEvent::Move {
            dx: parse_compact_i32(require_field(&fields, 1, "dx")?)?,
            dy: parse_compact_i32(require_field(&fields, 2, "dy")?)?,
        }),
        "D" => Ok(TouchEvent::MouseDelta {
            device_id: None,
            seq: parse_compact_u64(require_field(&fields, 1, "seq")?)?,
            dx: parse_compact_i32(require_field(&fields, 2, "dx")?)?,
            dy: parse_compact_i32(require_field(&fields, 3, "dy")?)?,
            dt: parse_compact_u32(require_field(&fields, 4, "dt")?)?,
            timestamp: None,
        }),
        "S" => Ok(TouchEvent::Scroll {
            dy: parse_compact_i32(require_field(&fields, 1, "dy")?)?,
        }),
        "C" => Ok(TouchEvent::Click {
            button: parse_mouse_button(require_field(&fields, 1, "button")?)?,
        }),
        "B" => Ok(TouchEvent::MouseButtonEvent {
            device_id: None,
            button: parse_mouse_button(require_field(&fields, 1, "button")?)?,
            action: parse_mouse_button_action(require_field(&fields, 2, "action")?)?,
            timestamp: None,
        }),
        "T" => Ok(TouchEvent::KeyboardText {
            device_id: None,
            seq: Some(parse_compact_u64(require_field(&fields, 1, "seq")?)?),
            text: decode_hex_utf8(require_field(&fields, 2, "text")?)?,
            timestamp: None,
        }),
        "K" => Ok(TouchEvent::KeyboardKey {
            device_id: None,
            seq: Some(parse_compact_u64(require_field(&fields, 1, "seq")?)?),
            key: parse_keyboard_key(require_field(&fields, 2, "key")?)?,
            modifiers: parse_keyboard_modifiers(fields.get(3).copied().unwrap_or_default())?,
            timestamp: None,
        }),
        "Y" => Ok(TouchEvent::CustomButtonEvent {
            device_id: None,
            button_id: decode_compact_field(require_field(&fields, 1, "button id")?)?,
            timestamp: None,
        }),
        "Q" => parse_custom_button_sync(&fields),
        _ => Err(format!("unknown compact command: {command}")),
    }
}

pub fn encode_compact_field(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.as_bytes() {
        let char = *byte as char;
        if char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.') {
            encoded.push(char);
        } else {
            encoded.push('%');
            encoded.push(HEX_DIGITS[(byte >> 4) as usize]);
            encoded.push(HEX_DIGITS[(byte & 0x0F) as usize]);
        }
    }

    encoded
}

impl TouchEvent {
    pub fn is_realtime_input(&self) -> bool {
        matches!(
            self,
            TouchEvent::Move { .. }
                | TouchEvent::Scroll { .. }
                | TouchEvent::MouseDelta { .. }
                | TouchEvent::KeyboardText { .. }
                | TouchEvent::KeyboardKey { .. }
        )
    }

    pub fn summary(&self) -> String {
        match self {
            TouchEvent::Move { dx, dy } => format!("move dx={dx}, dy={dy}"),
            TouchEvent::Click { button } => match button {
                MouseButton::Left => "click button=left".to_string(),
                MouseButton::Right => "click button=right".to_string(),
            },
            TouchEvent::Scroll { dy } => format!("scroll dy={dy}"),
            TouchEvent::MouseDelta {
                device_id,
                dx,
                dy,
                dt,
                seq,
                timestamp,
            } => format!(
                "mouse_delta device_id={}, dx={dx}, dy={dy}, dt={dt}, seq={seq}, timestamp={}",
                device_id.as_deref().unwrap_or("unknown"),
                optional_u64(timestamp)
            ),
            TouchEvent::KeyboardText {
                device_id,
                text,
                seq,
                timestamp,
            } => format!(
                "keyboard_text device_id={}, chars={}, seq={}, timestamp={}",
                device_id.as_deref().unwrap_or("unknown"),
                text.chars().count(),
                optional_u64(seq),
                optional_u64(timestamp)
            ),
            TouchEvent::KeyboardKey {
                device_id,
                key,
                modifiers,
                seq,
                timestamp,
            } => {
                let keys = if modifiers.is_empty() {
                    key.label().to_string()
                } else {
                    let mut keys = modifiers.clone();
                    keys.push(*key);
                    keys_to_text(&keys)
                };
                format!(
                    "keyboard_key device_id={}, key={keys}, seq={}, timestamp={}",
                    device_id.as_deref().unwrap_or("unknown"),
                    optional_u64(seq),
                    optional_u64(timestamp)
                )
            }
            TouchEvent::GestureEvent {
                device_id,
                gesture,
                profile_id,
                timestamp,
            } => format!(
                "gesture_event device_id={}, gesture={}, profile_id={}, timestamp={}",
                device_id.as_deref().unwrap_or("unknown"),
                gesture.event_name(),
                profile_id.as_deref().unwrap_or("default"),
                optional_u64(timestamp)
            ),
            TouchEvent::Handshake {
                device_id,
                client,
                protocol_version,
                timestamp,
            } => format!(
                "handshake device_id={device_id}, client={}, protocol_version={}, timestamp={}",
                client.as_deref().unwrap_or("unknown"),
                protocol_version
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                optional_u64(timestamp)
            ),
            TouchEvent::CustomButtonSyncBegin { device_id } => {
                format!(
                    "custom_button_sync_begin device_id={}",
                    device_id.as_deref().unwrap_or("unknown")
                )
            }
            TouchEvent::CustomButtonSyncItem { device_id, button } => format!(
                "custom_button_sync_item device_id={}, id={}, label={}, position={}",
                device_id.as_deref().unwrap_or("unknown"),
                button.id,
                button.label,
                button
                    .position
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            TouchEvent::CustomButtonSyncEnd { device_id } => {
                format!(
                    "custom_button_sync_end device_id={}",
                    device_id.as_deref().unwrap_or("unknown")
                )
            }
            TouchEvent::CustomButtonEvent {
                device_id,
                button_id,
                timestamp,
            } => format!(
                "custom_button_event device_id={}, button_id={button_id}, timestamp={}",
                device_id.as_deref().unwrap_or("unknown"),
                optional_u64(timestamp)
            ),
            TouchEvent::MouseButtonEvent {
                device_id,
                button,
                action,
                timestamp,
            } => {
                let button = match button {
                    MouseButton::Left => "left",
                    MouseButton::Right => "right",
                };
                let action = match action {
                    MouseButtonAction::Down => "down",
                    MouseButtonAction::Up => "up",
                };
                format!(
                    "mouse_button_event device_id={}, button={button}, action={action}, timestamp={}",
                    device_id.as_deref().unwrap_or("unknown"),
                    optional_u64(timestamp)
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gesture {
    Tap,
    DoubleTap,
    LongPress,
    SwipeUp,
    SwipeDown,
    SwipeLeft,
    SwipeRight,
    TwoFingerTap,
    TwoFingerSwipeLeft,
    TwoFingerSwipeRight,
    ThreeFingerTap,
}

impl Gesture {
    pub const MVP: [Gesture; 11] = [
        Gesture::Tap,
        Gesture::DoubleTap,
        Gesture::LongPress,
        Gesture::SwipeUp,
        Gesture::SwipeDown,
        Gesture::SwipeLeft,
        Gesture::SwipeRight,
        Gesture::TwoFingerTap,
        Gesture::TwoFingerSwipeLeft,
        Gesture::TwoFingerSwipeRight,
        Gesture::ThreeFingerTap,
    ];

    pub fn event_name(self) -> &'static str {
        match self {
            Gesture::Tap => "tap",
            Gesture::DoubleTap => "double_tap",
            Gesture::LongPress => "long_press",
            Gesture::SwipeUp => "swipe_up",
            Gesture::SwipeDown => "swipe_down",
            Gesture::SwipeLeft => "swipe_left",
            Gesture::SwipeRight => "swipe_right",
            Gesture::TwoFingerTap => "two_finger_tap",
            Gesture::TwoFingerSwipeLeft => "two_finger_swipe_left",
            Gesture::TwoFingerSwipeRight => "two_finger_swipe_right",
            Gesture::ThreeFingerTap => "three_finger_tap",
        }
    }

    pub fn label_for(self, language: AppLanguage) -> &'static str {
        match language {
            AppLanguage::English => self.english_label(),
            AppLanguage::Korean => self.korean_label(),
        }
    }

    fn english_label(self) -> &'static str {
        match self {
            Gesture::Tap => "Tap",
            Gesture::DoubleTap => "Double Tap",
            Gesture::LongPress => "Long Press",
            Gesture::SwipeUp => "Swipe Up",
            Gesture::SwipeDown => "Swipe Down",
            Gesture::SwipeLeft => "Swipe Left",
            Gesture::SwipeRight => "Swipe Right",
            Gesture::TwoFingerTap => "Two Finger Tap",
            Gesture::TwoFingerSwipeLeft => "Two Finger Swipe Left",
            Gesture::TwoFingerSwipeRight => "Two Finger Swipe Right",
            Gesture::ThreeFingerTap => "Three Finger Tap",
        }
    }

    pub fn description_for(self, language: AppLanguage) -> &'static str {
        match language {
            AppLanguage::English => self.english_description(),
            AppLanguage::Korean => self.korean_description(),
        }
    }

    fn english_description(self) -> &'static str {
        match self {
            Gesture::Tap => "one finger short tap",
            Gesture::DoubleTap => "one finger two quick taps",
            Gesture::LongPress => "one finger hold",
            Gesture::SwipeUp => "one finger upward swipe",
            Gesture::SwipeDown => "one finger downward swipe",
            Gesture::SwipeLeft => "one finger left swipe",
            Gesture::SwipeRight => "one finger right swipe",
            Gesture::TwoFingerTap => "two fingers short tap",
            Gesture::TwoFingerSwipeLeft => "two fingers left swipe",
            Gesture::TwoFingerSwipeRight => "two fingers right swipe",
            Gesture::ThreeFingerTap => "three fingers short tap",
        }
    }

    fn korean_label(self) -> &'static str {
        match self {
            Gesture::Tap => "탭",
            Gesture::DoubleTap => "더블 탭",
            Gesture::LongPress => "롱 프레스",
            Gesture::SwipeUp => "위로 스와이프",
            Gesture::SwipeDown => "아래로 스와이프",
            Gesture::SwipeLeft => "왼쪽 스와이프",
            Gesture::SwipeRight => "오른쪽 스와이프",
            Gesture::TwoFingerTap => "두 손가락 탭",
            Gesture::TwoFingerSwipeLeft => "두 손가락 왼쪽",
            Gesture::TwoFingerSwipeRight => "두 손가락 오른쪽",
            Gesture::ThreeFingerTap => "세 손가락 탭",
        }
    }

    fn korean_description(self) -> &'static str {
        match self {
            Gesture::Tap => "한 손가락 짧은 탭",
            Gesture::DoubleTap => "한 손가락 빠른 두 번 탭",
            Gesture::LongPress => "한 손가락 길게 누르기",
            Gesture::SwipeUp => "한 손가락 위쪽 스와이프",
            Gesture::SwipeDown => "한 손가락 아래쪽 스와이프",
            Gesture::SwipeLeft => "한 손가락 왼쪽 스와이프",
            Gesture::SwipeRight => "한 손가락 오른쪽 스와이프",
            Gesture::TwoFingerTap => "두 손가락 짧은 탭",
            Gesture::TwoFingerSwipeLeft => "두 손가락 왼쪽 스와이프",
            Gesture::TwoFingerSwipeRight => "두 손가락 오른쪽 스와이프",
            Gesture::ThreeFingerTap => "세 손가락 짧은 탭",
        }
    }
}

fn parse_custom_button_sync(fields: &[&str]) -> std::result::Result<TouchEvent, String> {
    match require_field(fields, 1, "custom sync command")? {
        "B" => Ok(TouchEvent::CustomButtonSyncBegin { device_id: None }),
        "E" => Ok(TouchEvent::CustomButtonSyncEnd { device_id: None }),
        "I" => Ok(TouchEvent::CustomButtonSyncItem {
            device_id: None,
            button: CustomButtonDefinition {
                position: Some(parse_compact_usize(require_field(fields, 2, "position")?)?),
                id: decode_compact_field(require_field(fields, 3, "button id")?)?,
                label: decode_compact_field(require_field(fields, 4, "button label")?)?,
            },
        }),
        other => Err(format!("unknown custom sync command: {other}")),
    }
}

fn parse_gesture(value: &str) -> std::result::Result<Gesture, String> {
    match value {
        "0" => Ok(Gesture::Tap),
        "1" => Ok(Gesture::DoubleTap),
        "2" => Ok(Gesture::LongPress),
        "3" => Ok(Gesture::SwipeUp),
        "4" => Ok(Gesture::SwipeDown),
        "5" => Ok(Gesture::SwipeLeft),
        "6" => Ok(Gesture::SwipeRight),
        "7" => Ok(Gesture::TwoFingerTap),
        "8" => Ok(Gesture::TwoFingerSwipeLeft),
        "9" => Ok(Gesture::TwoFingerSwipeRight),
        "a" | "A" => Ok(Gesture::ThreeFingerTap),
        _ => Err(format!("unknown gesture code: {value}")),
    }
}

fn parse_mouse_button(value: &str) -> std::result::Result<MouseButton, String> {
    match value {
        "L" | "l" => Ok(MouseButton::Left),
        "R" | "r" => Ok(MouseButton::Right),
        _ => Err(format!("unknown mouse button: {value}")),
    }
}

fn parse_mouse_button_action(value: &str) -> std::result::Result<MouseButtonAction, String> {
    match value {
        "D" | "d" => Ok(MouseButtonAction::Down),
        "U" | "u" => Ok(MouseButtonAction::Up),
        _ => Err(format!("unknown mouse button action: {value}")),
    }
}

fn parse_keyboard_key(value: &str) -> std::result::Result<KeyCode, String> {
    match value {
        "B" | "b" => Ok(KeyCode::Backspace),
        "E" | "e" => Ok(KeyCode::Enter),
        other => KeyCode::from_token(other).ok_or_else(|| format!("unknown keyboard key: {value}")),
    }
}

fn parse_keyboard_modifiers(value: &str) -> std::result::Result<Vec<KeyCode>, String> {
    let mut modifiers = Vec::new();

    for char in value.chars() {
        let modifier = match char {
            'C' | 'c' => KeyCode::Ctrl,
            'A' | 'a' => KeyCode::Alt,
            'S' | 's' => KeyCode::Shift,
            'W' | 'w' => KeyCode::Win,
            _ => return Err(format!("unknown keyboard modifier: {char}")),
        };

        if !modifiers.contains(&modifier) {
            modifiers.push(modifier);
        }
    }

    Ok(modifiers)
}

fn require_field<'a>(
    fields: &'a [&str],
    index: usize,
    label: &str,
) -> std::result::Result<&'a str, String> {
    fields
        .get(index)
        .copied()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("missing compact field: {label}"))
}

fn parse_compact_i32(value: &str) -> std::result::Result<i32, String> {
    if let Some(digits) = value.strip_prefix('-') {
        let parsed = i32::from_str_radix(digits, 36)
            .map_err(|err| format!("invalid compact integer {value}: {err}"))?;
        Ok(-parsed)
    } else {
        i32::from_str_radix(value, 36)
            .map_err(|err| format!("invalid compact integer {value}: {err}"))
    }
}

fn parse_compact_u32(value: &str) -> std::result::Result<u32, String> {
    u32::from_str_radix(value, 36)
        .map_err(|err| format!("invalid compact unsigned integer {value}: {err}"))
}

fn parse_compact_u64(value: &str) -> std::result::Result<u64, String> {
    u64::from_str_radix(value, 36)
        .map_err(|err| format!("invalid compact unsigned integer {value}: {err}"))
}

fn parse_compact_usize(value: &str) -> std::result::Result<usize, String> {
    usize::from_str_radix(value, 36)
        .map_err(|err| format!("invalid compact position {value}: {err}"))
}

fn decode_hex_utf8(value: &str) -> std::result::Result<String, String> {
    if value.len() % 2 != 0 {
        return Err("hex text has odd length".to_string());
    }

    let mut bytes = Vec::with_capacity(value.len() / 2);
    let chars = value.as_bytes();
    for index in (0..chars.len()).step_by(2) {
        let high = hex_value(chars[index]).ok_or_else(|| "invalid hex text".to_string())?;
        let low = hex_value(chars[index + 1]).ok_or_else(|| "invalid hex text".to_string())?;
        bytes.push((high << 4) | low);
    }

    String::from_utf8(bytes).map_err(|err| format!("invalid utf-8 text: {err}"))
}

fn decode_compact_field(value: &str) -> std::result::Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).map_err(|err| format!("invalid compact field utf-8: {err}"))
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn optional_u64(value: &Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

const HEX_DIGITS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F',
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_keyboard_key_without_modifiers() {
        match parse_compact_event("K:1:Left").expect("event") {
            TouchEvent::KeyboardKey {
                key,
                modifiers,
                seq,
                ..
            } => {
                assert_eq!(key, KeyCode::Left);
                assert!(modifiers.is_empty());
                assert_eq!(seq, Some(1));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_keyboard_key_with_modifiers() {
        match parse_compact_event("K:2:KeyC:C").expect("event") {
            TouchEvent::KeyboardKey {
                key,
                modifiers,
                seq,
                ..
            } => {
                assert_eq!(key, KeyCode::C);
                assert_eq!(modifiers, vec![KeyCode::Ctrl]);
                assert_eq!(seq, Some(2));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parses_multiple_keyboard_modifiers_in_payload_order() {
        match parse_compact_event("K:3:Right:CSW").expect("event") {
            TouchEvent::KeyboardKey { key, modifiers, .. } => {
                assert_eq!(key, KeyCode::Right);
                assert_eq!(modifiers, vec![KeyCode::Ctrl, KeyCode::Shift, KeyCode::Win]);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
