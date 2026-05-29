use serde::{Deserialize, Serialize};

use crate::i18n::AppLanguage;

/// BLE GATT characteristic로 들어오는 JSON 이벤트를 Rust enum으로 표현합니다.
///
/// serde의 `tag = "type"` 옵션을 사용하면 아래 JSON의 `type` 값에 따라
/// 자동으로 Move, Click, Scroll 중 하나로 역직렬화됩니다.
///
/// {"type":"move","dx":30,"dy":-10}
/// {"type":"click","button":"left"}
/// {"type":"scroll","dy":-120}
/// {"type":"hotkey","name":"task_manager"}
/// {"type":"gesture","name":"swipe_left"}
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
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
    Hotkey {
        name: HotkeyName,
    },
    Gesture {
        name: Gesture,
    },
    #[serde(rename = "gesture_event")]
    GestureEvent {
        #[serde(rename = "deviceId")]
        device_id: String,
        gesture: Gesture,
        #[serde(rename = "profileId")]
        profile_id: Option<String>,
        timestamp: Option<u64>,
    },
    #[serde(rename = "handshake")]
    Handshake {
        #[serde(rename = "deviceId")]
        device_id: String,
        client: Option<String>,
        #[serde(rename = "protocolVersion")]
        protocol_version: Option<u32>,
        timestamp: Option<u64>,
    },
    #[serde(rename = "custom_button_sync")]
    CustomButtonSync {
        #[serde(rename = "deviceId")]
        device_id: String,
        buttons: Vec<CustomButtonDefinition>,
        timestamp: Option<u64>,
    },
    #[serde(rename = "custom_button_event")]
    CustomButtonEvent {
        #[serde(rename = "deviceId")]
        device_id: String,
        #[serde(rename = "buttonId")]
        button_id: String,
        timestamp: Option<u64>,
    },
}

/// 현재 MVP에서는 왼쪽/오른쪽 클릭만 지원합니다.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
}

/// 로컬 PC에서 실행할 미리 정해진 단축키입니다.
///
/// MVP에서는 임의 키 조합을 모두 허용하지 않고, 안전하게 테스트할 수 있는
/// 작업 관리자 단축키만 먼저 노출합니다.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyName {
    TaskManager,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CustomButtonDefinition {
    pub id: String,
    pub label: String,
    pub position: Option<usize>,
}

impl TouchEvent {
    pub fn summary(&self) -> String {
        match self {
            TouchEvent::Move { dx, dy } => format!("move dx={dx}, dy={dy}"),
            TouchEvent::Click { button } => match button {
                MouseButton::Left => "click button=left".to_string(),
                MouseButton::Right => "click button=right".to_string(),
            },
            TouchEvent::Scroll { dy } => format!("scroll dy={dy}"),
            TouchEvent::Hotkey { name } => match name {
                HotkeyName::TaskManager => "hotkey name=task_manager -> Ctrl+Shift+Esc".to_string(),
            },
            TouchEvent::Gesture { name } => format!("gesture name={}", name.event_name()),
            TouchEvent::GestureEvent {
                device_id,
                gesture,
                profile_id,
                timestamp,
            } => format!(
                "gesture_event device_id={device_id}, gesture={}, profile_id={}, timestamp={}",
                gesture.event_name(),
                profile_id.as_deref().unwrap_or("default"),
                timestamp
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
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
                timestamp
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            TouchEvent::CustomButtonSync {
                device_id,
                buttons,
                timestamp,
            } => format!(
                "custom_button_sync device_id={device_id}, buttons={}, timestamp={}",
                buttons.len(),
                timestamp
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            TouchEvent::CustomButtonEvent {
                device_id,
                button_id,
                timestamp,
            } => format!(
                "custom_button_event device_id={device_id}, button_id={button_id}, timestamp={}",
                timestamp
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        }
    }
}

/// MVP에서 GUI에 노출할 제스처 목록입니다.
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
