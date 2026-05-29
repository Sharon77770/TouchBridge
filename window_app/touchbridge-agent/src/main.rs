mod action;
mod ble;
mod config;
mod dispatch;
mod event;
mod gui;
mod i18n;
mod input;
mod protocol;
mod tray;
mod usb;

use std::sync::{Arc, RwLock};
use std::thread;

use config::{AppConfig, AppState, default_config_path};
use i18n::{ble_service_stopped, usb_service_stopped};

fn main() -> eframe::Result<()> {
    println!("Starting TouchBridge Windows Agent BLE MVP");

    let config_path = default_config_path();
    let config = AppConfig::load_or_default(&config_path);
    let language = config.language;
    let state = Arc::new(RwLock::new(AppState::new(config)));

    start_ble_thread(state.clone());
    start_usb_thread(state.clone());
    let tray = tray::TrayHandle::start(language);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([960.0, 620.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "TouchBridge Agent",
        options,
        Box::new(|_cc| Ok(Box::new(gui::TouchBridgeGui::new(state, config_path, tray)))),
    )
}

fn start_ble_thread(state: config::SharedAppState) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

        if let Err(err) = runtime.block_on(ble::run(state.clone())) {
            eprintln!("BLE service stopped: {err}");

            let mut state = state.write().expect("app state poisoned");
            state.ble_status = ble_service_stopped(state.config.language).to_string();
            state.ble_error = Some(err.to_string());
        }
    });
}

fn start_usb_thread(state: config::SharedAppState) {
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

        if let Err(err) = runtime.block_on(usb::run(state.clone())) {
            eprintln!("USB service stopped: {err}");

            let mut state = state.write().expect("app state poisoned");
            state.usb_status = usb_service_stopped(state.config.language).to_string();
            state.usb_error = Some(err.to_string());
        }
    });
}
