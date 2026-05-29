use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::action::{Action, KeyCode};
use crate::event::{CustomButtonDefinition, Gesture};
use crate::i18n::{self, AppLanguage, TextKey};

pub type SharedAppState = Arc<RwLock<AppState>>;

#[derive(Debug)]
pub struct AppState {
    pub config: AppConfig,
    pub last_request: String,
    pub last_action: String,
    pub last_error: Option<String>,
    pub ble_status: String,
    pub ble_error: Option<String>,
    pub usb_status: String,
    pub usb_error: Option<String>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let language = config.language;

        Self {
            config,
            last_request: i18n::text(language, TextKey::NoRequestYet).to_string(),
            last_action: i18n::text(language, TextKey::Idle).to_string(),
            last_error: None,
            ble_status: i18n::text(language, TextKey::BleServiceNotStarted).to_string(),
            ble_error: None,
            usb_status: i18n::text(language, TextKey::UsbServiceNotStarted).to_string(),
            usb_error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub language: AppLanguage,
    #[serde(default = "default_mappings")]
    pub mappings: Vec<GestureMapping>,
    #[serde(default)]
    pub custom_buttons: Vec<CustomButtonMapping>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GestureMapping {
    pub gesture: Gesture,
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomButtonMapping {
    pub id: String,
    pub label: String,
    pub position: usize,
    pub action: Action,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: AppLanguage::default(),
            mappings: default_mappings(),
            custom_buttons: Vec::new(),
        }
    }
}

fn default_mappings() -> Vec<GestureMapping> {
    use Gesture::*;
    use KeyCode::*;

    vec![
        GestureMapping {
            gesture: Tap,
            action: Action::hotkey(vec![Enter]),
        },
        GestureMapping {
            gesture: DoubleTap,
            action: Action::hotkey(vec![Ctrl, W]),
        },
        GestureMapping {
            gesture: LongPress,
            action: Action::hotkey(vec![Ctrl, Shift, Esc]),
        },
        GestureMapping {
            gesture: SwipeUp,
            action: Action::hotkey(vec![Win, Tab]),
        },
        GestureMapping {
            gesture: SwipeDown,
            action: Action::hotkey(vec![Win, D]),
        },
        GestureMapping {
            gesture: SwipeLeft,
            action: Action::hotkey(vec![Alt, Left]),
        },
        GestureMapping {
            gesture: SwipeRight,
            action: Action::hotkey(vec![Alt, Right]),
        },
        GestureMapping {
            gesture: TwoFingerTap,
            action: Action::hotkey(vec![Alt, Tab]),
        },
        GestureMapping {
            gesture: TwoFingerSwipeLeft,
            action: Action::hotkey(vec![Ctrl, Win, Left]),
        },
        GestureMapping {
            gesture: TwoFingerSwipeRight,
            action: Action::hotkey(vec![Ctrl, Win, Right]),
        },
        GestureMapping {
            gesture: ThreeFingerTap,
            action: Action::hotkey(vec![Ctrl, Shift, T]),
        },
    ]
}

impl AppConfig {
    pub fn action_for(&self, gesture: Gesture) -> Action {
        self.mappings
            .iter()
            .find(|mapping| mapping.gesture == gesture)
            .map(|mapping| mapping.action.clone())
            .unwrap_or(Action::None)
    }

    pub fn set_action(&mut self, gesture: Gesture, action: Action) {
        if let Some(mapping) = self
            .mappings
            .iter_mut()
            .find(|mapping| mapping.gesture == gesture)
        {
            mapping.action = action;
        } else {
            self.mappings.push(GestureMapping { gesture, action });
        }
    }

    pub fn action_for_custom_button(&self, button_id: &str) -> Action {
        self.custom_buttons
            .iter()
            .find(|button| button.id == button_id)
            .map(|button| button.action.clone())
            .unwrap_or(Action::None)
    }

    pub fn set_custom_button_action(&mut self, button_id: &str, action: Action) {
        if let Some(button) = self
            .custom_buttons
            .iter_mut()
            .find(|button| button.id == button_id)
        {
            button.action = action;
        }
    }

    pub fn sync_custom_buttons(&mut self, buttons: &[CustomButtonDefinition]) {
        let mut next = Vec::new();

        for (index, incoming) in buttons.iter().enumerate() {
            let id = incoming.id.trim();
            let label = incoming.label.trim();

            if id.is_empty() || label.is_empty() {
                continue;
            }

            let existing_action = self
                .custom_buttons
                .iter()
                .find(|button| button.id == id)
                .map(|button| button.action.clone())
                .unwrap_or(Action::None);

            next.push(CustomButtonMapping {
                id: id.to_string(),
                label: label.to_string(),
                position: incoming.position.unwrap_or(index),
                action: existing_action,
            });
        }

        next.sort_by(|left, right| {
            left.position
                .cmp(&right.position)
                .then_with(|| left.label.cmp(&right.label))
        });

        for (index, button) in next.iter_mut().enumerate() {
            button.position = index;
        }

        self.custom_buttons = next;
    }

    pub fn load_or_default(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|err| {
                eprintln!("Failed to parse config, using defaults: {err}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let raw = serde_json::to_string_pretty(self)?;
        fs::write(path, raw)
    }
}

pub fn default_config_path() -> PathBuf {
    PathBuf::from("touchbridge-config.json")
}
