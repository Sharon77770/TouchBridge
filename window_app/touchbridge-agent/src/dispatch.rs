use crate::config::SharedAppState;
use crate::event::{TouchEvent, encode_compact_field, parse_compact_event};
use crate::i18n::{self, TextKey};
use crate::input;

pub struct CommandAck {
    pub ok: bool,
    pub message: String,
}

impl CommandAck {
    pub fn executed() -> Self {
        Self {
            ok: true,
            message: "executed".to_string(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: message.into(),
        }
    }

    pub fn to_compact(&self) -> String {
        if self.ok {
            "A1".to_string()
        } else {
            format!("A0:{}", encode_compact_field(&self.message))
        }
    }
}

pub fn handle_raw_message(raw: &str, transport: &str, state: &SharedAppState) -> CommandAck {
    let raw = raw.trim();
    let language = {
        let state = state.read().expect("app state poisoned");
        state.config.language
    };

    if raw.is_empty() {
        return CommandAck::failed(i18n::invalid_request(language, "empty request"));
    }

    match parse_compact_event(raw) {
        Ok(event) => {
            if !event.is_realtime_input() {
                println!("{transport} request: {raw}");
            }
            handle_event(raw, event, state)
        }
        Err(err) => {
            eprintln!("Invalid compact event: {err}; raw={raw}");

            let mut state = state.write().expect("app state poisoned");
            state.last_request = raw.to_string();
            state.last_action = i18n::text(language, TextKey::InvalidRequest).to_string();
            state.last_error = Some(err.to_string());

            CommandAck::failed(i18n::invalid_request(language, err))
        }
    }
}

fn handle_event(raw: &str, event: TouchEvent, state: &SharedAppState) -> CommandAck {
    let realtime_input = event.is_realtime_input();
    let language = {
        let state = state.read().expect("app state poisoned");
        state.config.language
    };

    match &event {
        TouchEvent::Handshake { .. } => {
            let mut state = state.write().expect("app state poisoned");
            state.last_request = raw.to_string();
            state.last_action = event.summary();
            state.last_error = None;
            return CommandAck::executed();
        }
        TouchEvent::CustomButtonSyncBegin { .. } => {
            let mut state = state.write().expect("app state poisoned");
            state.pending_custom_buttons.clear();
            state.last_request = raw.to_string();
            state.last_action = event.summary();
            state.last_error = None;
            return CommandAck::executed();
        }
        TouchEvent::CustomButtonSyncItem { button, .. } => {
            let mut state = state.write().expect("app state poisoned");
            state.pending_custom_buttons.push(button.clone());
            state.last_request = raw.to_string();
            state.last_action = event.summary();
            state.last_error = None;
            return CommandAck::executed();
        }
        TouchEvent::CustomButtonSyncEnd { .. } => {
            let mut state = state.write().expect("app state poisoned");
            let buttons = std::mem::take(&mut state.pending_custom_buttons);
            state.config.sync_custom_buttons(&buttons);
            state.last_request = raw.to_string();
            state.last_action = match language {
                crate::i18n::AppLanguage::English => {
                    format!("Synced {} custom button(s)", buttons.len())
                }
                crate::i18n::AppLanguage::Korean => {
                    format!("커스텀 버튼 {}개 동기화됨", buttons.len())
                }
            };
            state.last_error = None;
            return CommandAck::executed();
        }
        _ => {}
    }

    let result = match &event {
        TouchEvent::GestureEvent { gesture: name, .. } => {
            let action = {
                let state = state.read().expect("app state poisoned");
                state.config.action_for(*name)
            };

            println!("Action: {} -> {}", event.summary(), action.summary());
            input::send_action(&action).map(|_| action.summary_for(language))
        }
        TouchEvent::CustomButtonEvent { button_id, .. } => {
            let action = {
                let state = state.read().expect("app state poisoned");
                state.config.action_for_custom_button(button_id)
            };

            println!("Action: {} -> {}", event.summary(), action.summary());
            input::send_action(&action).map(|_| action.summary_for(language))
        }
        _ => {
            if !realtime_input {
                println!("Action: {}", event.summary());
            }
            input::send_event(&event).map(|_| event.summary())
        }
    };

    if realtime_input {
        return match result {
            Ok(_) => CommandAck::executed(),
            Err(err) => {
                eprintln!("Failed to send realtime Windows input: {err}");
                CommandAck::failed(i18n::input_failed(language, err))
            }
        };
    }

    let mut state = state.write().expect("app state poisoned");
    state.last_request = raw.to_string();

    match result {
        Ok(action_summary) => {
            println!("Windows input sent: {action_summary}");
            state.last_action = action_summary;
            state.last_error = None;
            CommandAck::executed()
        }
        Err(err) => {
            state.last_action = i18n::text(language, TextKey::InputFailed).to_string();
            state.last_error = Some(err.to_string());
            eprintln!("Failed to send Windows input: {err}");
            CommandAck::failed(i18n::input_failed(language, err))
        }
    }
}
