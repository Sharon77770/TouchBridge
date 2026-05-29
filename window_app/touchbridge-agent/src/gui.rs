use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use eframe::egui;

use crate::action::{ActionKind, parse_action};
use crate::config::{AppConfig, SharedAppState};
use crate::event::Gesture;
use crate::i18n::{self, AppLanguage, TextKey};
use crate::input;
use crate::protocol::{GESTURE_CHARACTERISTIC_UUID_STRING, SERVICE_UUID_STRING};
use crate::tray::TrayHandle;

pub struct TouchBridgeGui {
    state: SharedAppState,
    config_path: PathBuf,
    action_inputs: HashMap<Gesture, String>,
    action_kinds: HashMap<Gesture, ActionKind>,
    input_errors: HashMap<Gesture, String>,
    custom_action_inputs: HashMap<String, String>,
    custom_action_kinds: HashMap<String, ActionKind>,
    custom_input_errors: HashMap<String, String>,
    save_status: String,
    fonts_configured: bool,
    tray: TrayHandle,
    allow_exit: bool,
}

impl TouchBridgeGui {
    pub fn new(
        state: SharedAppState,
        config_path: PathBuf,
        tray: TrayHandle,
        repaint_context: egui::Context,
    ) -> Self {
        tray.set_repaint_context(repaint_context);

        let (action_inputs, action_kinds) = {
            let state = state.read().expect("app state poisoned");
            let mut inputs = HashMap::new();
            let mut kinds = HashMap::new();

            for gesture in Gesture::MVP {
                let action = state.config.action_for(gesture);
                inputs.insert(gesture, action.display_text());
                kinds.insert(gesture, action.kind());
            }

            (inputs, kinds)
        };

        let language = {
            let state = state.read().expect("app state poisoned");
            state.config.language
        };
        let (custom_action_inputs, custom_action_kinds) = {
            let state = state.read().expect("app state poisoned");
            let mut inputs = HashMap::new();
            let mut kinds = HashMap::new();

            for button in &state.config.custom_buttons {
                inputs.insert(button.id.clone(), button.action.display_text());
                kinds.insert(button.id.clone(), button.action.kind());
            }

            (inputs, kinds)
        };

        Self {
            state,
            config_path,
            action_inputs,
            action_kinds,
            input_errors: HashMap::new(),
            custom_action_inputs,
            custom_action_kinds,
            custom_input_errors: HashMap::new(),
            save_status: i18n::text(language, TextKey::SettingsInMemory).to_string(),
            fonts_configured: false,
            tray,
            allow_exit: false,
        }
    }

    fn language(&self) -> AppLanguage {
        let state = self.state.read().expect("app state poisoned");
        state.config.language
    }

    fn set_language(&mut self, language: AppLanguage) {
        {
            let mut state = self.state.write().expect("app state poisoned");
            state.config.language = language;
        }
        self.save_status = i18n::text(language, TextKey::SettingsInMemory).to_string();
    }

    fn set_mapping_from_text(&mut self, gesture: Gesture) {
        let value = self
            .action_inputs
            .get(&gesture)
            .cloned()
            .unwrap_or_default();
        let kind = self
            .action_kinds
            .get(&gesture)
            .copied()
            .unwrap_or(ActionKind::Hotkey);

        match parse_action(kind, &value) {
            Ok(action) => {
                self.input_errors.remove(&gesture);

                let mut state = self.state.write().expect("app state poisoned");
                state.config.set_action(gesture, action);
            }
            Err(err) => {
                self.input_errors
                    .insert(gesture, i18n::parse_error(self.language(), &err));
            }
        }
    }

    fn set_mapping_kind(&mut self, gesture: Gesture, kind: ActionKind) {
        let previous_kind = self
            .action_kinds
            .insert(gesture, kind)
            .unwrap_or(ActionKind::Hotkey);

        if previous_kind != kind {
            self.action_inputs.insert(gesture, String::new());
        }

        self.set_mapping_from_text(gesture);
    }

    fn set_custom_mapping_from_text(&mut self, button_id: &str) {
        let value = self
            .custom_action_inputs
            .get(button_id)
            .cloned()
            .unwrap_or_default();
        let kind = self
            .custom_action_kinds
            .get(button_id)
            .copied()
            .unwrap_or(ActionKind::None);

        match parse_action(kind, &value) {
            Ok(action) => {
                self.custom_input_errors.remove(button_id);

                let mut state = self.state.write().expect("app state poisoned");
                state.config.set_custom_button_action(button_id, action);
            }
            Err(err) => {
                self.custom_input_errors.insert(
                    button_id.to_string(),
                    i18n::parse_error(self.language(), &err),
                );
            }
        }
    }

    fn set_custom_mapping_kind(&mut self, button_id: &str, kind: ActionKind) {
        let previous_kind = self
            .custom_action_kinds
            .insert(button_id.to_string(), kind)
            .unwrap_or(ActionKind::None);

        if previous_kind != kind {
            self.custom_action_inputs
                .insert(button_id.to_string(), String::new());
        }

        self.set_custom_mapping_from_text(button_id);
    }

    fn reconcile_custom_button_inputs(&mut self) {
        let buttons = {
            let state = self.state.read().expect("app state poisoned");
            state.config.custom_buttons.clone()
        };

        self.custom_action_inputs
            .retain(|id, _| buttons.iter().any(|button| button.id == *id));
        self.custom_action_kinds
            .retain(|id, _| buttons.iter().any(|button| button.id == *id));
        self.custom_input_errors
            .retain(|id, _| buttons.iter().any(|button| button.id == *id));

        for button in buttons {
            self.custom_action_kinds
                .entry(button.id.clone())
                .or_insert_with(|| button.action.kind());
            self.custom_action_inputs
                .entry(button.id)
                .or_insert_with(|| button.action.display_text());
        }
    }

    fn reset_to_defaults(&mut self) {
        let language = self.language();
        let mut config = AppConfig::default();
        config.language = language;

        {
            let mut state = self.state.write().expect("app state poisoned");
            state.config = config.clone();
            state.last_error = None;
        }

        self.action_inputs = Gesture::MVP
            .iter()
            .map(|gesture| (*gesture, config.action_for(*gesture).display_text()))
            .collect();
        self.action_kinds = Gesture::MVP
            .iter()
            .map(|gesture| (*gesture, config.action_for(*gesture).kind()))
            .collect();
        self.input_errors.clear();
        self.custom_action_inputs.clear();
        self.custom_action_kinds.clear();
        self.custom_input_errors.clear();
        self.save_status =
            i18n::text(self.language(), TextKey::DefaultsRestoredInMemory).to_string();
    }

    fn save_config(&mut self) {
        let config = {
            let state = self.state.read().expect("app state poisoned");
            state.config.clone()
        };

        match config.save(&self.config_path) {
            Ok(()) => {
                self.save_status = i18n::saved_to(config.language, self.config_path.display());
            }
            Err(err) => {
                self.save_status = i18n::save_failed(config.language, err);
            }
        }
    }

    fn configure_fonts(&mut self, ctx: &egui::Context) {
        if self.fonts_configured {
            return;
        }
        self.fonts_configured = true;

        let Some(font_bytes) = [
            r"C:\Windows\Fonts\malgun.ttf",
            r"C:\Windows\Fonts\malgunbd.ttf",
            r"C:\Windows\Fonts\gulim.ttc",
        ]
        .iter()
        .find_map(|path| fs::read(path).ok()) else {
            return;
        };

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "TouchBridgeKoreanFallback".to_owned(),
            Arc::new(egui::FontData::from_owned(font_bytes)),
        );

        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("TouchBridgeKoreanFallback".to_owned());
        }

        ctx.set_fonts(fonts);
    }
}

impl eframe::App for TouchBridgeGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tray.set_repaint_context(ctx.clone());
        self.configure_fonts(ctx);
        ctx.request_repaint_after(Duration::from_millis(250));

        if ctx.input(|input| input.viewport().close_requested())
            && !self.allow_exit
            && self.tray.is_available()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }

        if self.tray.take_show_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }

        if self.tray.take_exit_requested() {
            self.allow_exit = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        self.reconcile_custom_button_inputs();
        let language = self.language();
        self.tray.set_language(language);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TouchBridge Agent");
                ui.separator();
                ui.label("BLE GATT + USB AOA");
                ui.separator();
                ui.label(i18n::text(language, TextKey::Language));
                let mut selected_language = language;
                ui.radio_value(
                    &mut selected_language,
                    AppLanguage::English,
                    AppLanguage::English.label(),
                );
                ui.radio_value(
                    &mut selected_language,
                    AppLanguage::Korean,
                    AppLanguage::Korean.label(),
                );
                if selected_language != language {
                    self.set_language(selected_language);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(i18n::text(language, TextKey::Save)).clicked() {
                    self.save_config();
                }

                if ui
                    .button(i18n::text(language, TextKey::ResetDefaults))
                    .clicked()
                {
                    self.reset_to_defaults();
                }

                ui.label(&self.save_status);
            });

            ui.separator();

            let (
                last_request,
                last_action,
                last_error,
                ble_status,
                ble_error,
                usb_status,
                usb_error,
            ) = {
                let state = self.state.read().expect("app state poisoned");
                (
                    state.last_request.clone(),
                    state.last_action.clone(),
                    state.last_error.clone(),
                    state.ble_status.clone(),
                    state.ble_error.clone(),
                    state.usb_status.clone(),
                    state.usb_error.clone(),
                )
            };

            ui.label(format!(
                "{}: {last_request}",
                i18n::text(language, TextKey::LastRequest)
            ));
            ui.label(format!(
                "{}: {last_action}",
                i18n::text(language, TextKey::LastAction)
            ));
            ui.label(format!("BLE: {ble_status}"));
            ui.label(format!("USB: {usb_status}"));
            ui.label(format!(
                "{}: {SERVICE_UUID_STRING}",
                i18n::text(language, TextKey::ServiceUuid)
            ));
            ui.label(format!(
                "{}: {GESTURE_CHARACTERISTIC_UUID_STRING}",
                i18n::text(language, TextKey::GestureCharacteristicUuid)
            ));

            if let Some(error) = last_error {
                ui.colored_label(egui::Color32::from_rgb(190, 40, 40), error);
            }

            if let Some(error) = ble_error {
                ui.colored_label(egui::Color32::from_rgb(190, 40, 40), error);
            }

            if let Some(error) = usb_error {
                ui.colored_label(egui::Color32::from_rgb(190, 40, 40), error);
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(i18n::text(language, TextKey::Gesture));
                egui::Grid::new("gesture_mapping_grid")
                    .num_columns(6)
                    .striped(true)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.strong(i18n::text(language, TextKey::Gesture));
                        ui.strong(i18n::text(language, TextKey::Description));
                        ui.strong(i18n::text(language, TextKey::ActionType));
                        ui.strong(i18n::text(language, TextKey::ActionContent));
                        ui.strong(i18n::text(language, TextKey::Test));
                        ui.strong(i18n::text(language, TextKey::Status));
                        ui.end_row();

                        for gesture in Gesture::MVP {
                            ui.label(gesture.label_for(language));
                            ui.label(gesture.description_for(language));

                            let mut selected_kind = self
                                .action_kinds
                                .get(&gesture)
                                .copied()
                                .unwrap_or(ActionKind::Hotkey);
                            egui::ComboBox::from_id_salt((
                                "gesture_action_kind",
                                gesture.event_name(),
                            ))
                            .selected_text(selected_kind.label_for(language))
                            .show_ui(ui, |ui| {
                                for kind in ActionKind::ALL {
                                    ui.selectable_value(
                                        &mut selected_kind,
                                        kind,
                                        kind.label_for(language),
                                    );
                                }
                            });

                            if selected_kind
                                != self
                                    .action_kinds
                                    .get(&gesture)
                                    .copied()
                                    .unwrap_or(ActionKind::Hotkey)
                            {
                                self.set_mapping_kind(gesture, selected_kind);
                            }

                            let text = self.action_inputs.entry(gesture).or_default();
                            let response = match selected_kind {
                                ActionKind::None => {
                                    ui.label(i18n::text(language, TextKey::NoAction))
                                }
                                ActionKind::Hotkey => ui.add_sized(
                                    [180.0, 24.0],
                                    egui::TextEdit::singleline(text).hint_text("Ctrl+Shift+Esc"),
                                ),
                                ActionKind::PythonScript | ActionKind::PowerShellScript => ui
                                    .add_sized(
                                        [320.0, 84.0],
                                        egui::TextEdit::multiline(text)
                                            .desired_rows(3)
                                            .code_editor(),
                                    ),
                            };

                            if response.changed() {
                                self.set_mapping_from_text(gesture);
                            }

                            if ui.button(i18n::text(language, TextKey::Run)).clicked() {
                                self.set_mapping_from_text(gesture);

                                if !self.input_errors.contains_key(&gesture) {
                                    let action = {
                                        let state = self.state.read().expect("app state poisoned");
                                        state.config.action_for(gesture)
                                    };

                                    match input::send_action(&action) {
                                        Ok(()) => {
                                            let mut state =
                                                self.state.write().expect("app state poisoned");
                                            state.last_request =
                                                format!("GUI test: {}", gesture.event_name());
                                            state.last_action = action.summary_for(language);
                                            state.last_error = None;
                                        }
                                        Err(err) => {
                                            let mut state =
                                                self.state.write().expect("app state poisoned");
                                            state.last_error =
                                                Some(i18n::input_failed(language, err));
                                        }
                                    }
                                }
                            }

                            if let Some(error) = self.input_errors.get(&gesture) {
                                ui.colored_label(egui::Color32::from_rgb(190, 40, 40), error);
                            } else {
                                ui.label(i18n::text(language, TextKey::Ok));
                            }

                            ui.end_row();
                        }
                    });

                ui.add_space(18.0);
                ui.heading(i18n::text(language, TextKey::CustomButtons));

                let custom_buttons = {
                    let state = self.state.read().expect("app state poisoned");
                    state.config.custom_buttons.clone()
                };

                if custom_buttons.is_empty() {
                    ui.label(match language {
                        AppLanguage::English => {
                            "Connect the mobile app and add custom buttons to sync them here."
                        }
                        AppLanguage::Korean => {
                            "모바일 앱을 연결하고 커스텀 버튼을 추가하면 여기에 동기화됩니다."
                        }
                    });
                } else {
                    egui::Grid::new("custom_button_mapping_grid")
                        .num_columns(6)
                        .striped(true)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            ui.strong(i18n::text(language, TextKey::ButtonName));
                            ui.strong(i18n::text(language, TextKey::ButtonId));
                            ui.strong(i18n::text(language, TextKey::ActionType));
                            ui.strong(i18n::text(language, TextKey::ActionContent));
                            ui.strong(i18n::text(language, TextKey::Test));
                            ui.strong(i18n::text(language, TextKey::Status));
                            ui.end_row();

                            for button in custom_buttons {
                                let button_id = button.id.clone();
                                ui.label(button.label);
                                ui.label(&button_id);

                                let mut selected_kind = self
                                    .custom_action_kinds
                                    .get(&button_id)
                                    .copied()
                                    .unwrap_or(ActionKind::None);
                                egui::ComboBox::from_id_salt((
                                    "custom_button_action_kind",
                                    button_id.clone(),
                                ))
                                .selected_text(selected_kind.label_for(language))
                                .show_ui(ui, |ui| {
                                    for kind in ActionKind::ALL {
                                        ui.selectable_value(
                                            &mut selected_kind,
                                            kind,
                                            kind.label_for(language),
                                        );
                                    }
                                });

                                if selected_kind
                                    != self
                                        .custom_action_kinds
                                        .get(&button_id)
                                        .copied()
                                        .unwrap_or(ActionKind::None)
                                {
                                    self.set_custom_mapping_kind(&button_id, selected_kind);
                                }

                                let text = self
                                    .custom_action_inputs
                                    .entry(button_id.clone())
                                    .or_default();
                                let response = match selected_kind {
                                    ActionKind::None => {
                                        ui.label(i18n::text(language, TextKey::NoAction))
                                    }
                                    ActionKind::Hotkey => ui.add_sized(
                                        [180.0, 24.0],
                                        egui::TextEdit::singleline(text)
                                            .hint_text("Ctrl+Shift+Esc"),
                                    ),
                                    ActionKind::PythonScript | ActionKind::PowerShellScript => ui
                                        .add_sized(
                                            [320.0, 84.0],
                                            egui::TextEdit::multiline(text)
                                                .desired_rows(3)
                                                .code_editor(),
                                        ),
                                };

                                if response.changed() {
                                    self.set_custom_mapping_from_text(&button_id);
                                }

                                if ui.button(i18n::text(language, TextKey::Run)).clicked() {
                                    self.set_custom_mapping_from_text(&button_id);

                                    if !self.custom_input_errors.contains_key(&button_id) {
                                        let action = {
                                            let state =
                                                self.state.read().expect("app state poisoned");
                                            state.config.action_for_custom_button(&button_id)
                                        };

                                        match input::send_action(&action) {
                                            Ok(()) => {
                                                let mut state =
                                                    self.state.write().expect("app state poisoned");
                                                state.last_request =
                                                    format!("GUI custom button test: {button_id}");
                                                state.last_action = action.summary_for(language);
                                                state.last_error = None;
                                            }
                                            Err(err) => {
                                                let mut state =
                                                    self.state.write().expect("app state poisoned");
                                                state.last_error =
                                                    Some(i18n::input_failed(language, err));
                                            }
                                        }
                                    }
                                }

                                if let Some(error) = self.custom_input_errors.get(&button_id) {
                                    ui.colored_label(egui::Color32::from_rgb(190, 40, 40), error);
                                } else {
                                    ui.label(i18n::text(language, TextKey::Ok));
                                }

                                ui.end_row();
                            }
                        });
                }
            });
        });
    }
}
