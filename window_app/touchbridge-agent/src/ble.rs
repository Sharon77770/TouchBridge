use std::future::IntoFuture;
use std::sync::mpsc::{self, Sender};
use std::thread;

use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattLocalCharacteristic, GattLocalCharacteristicParameters,
    GattLocalCharacteristicResult, GattProtectionLevel, GattServiceProvider,
    GattServiceProviderAdvertisementStatus, GattServiceProviderAdvertisementStatusChangedEventArgs,
    GattServiceProviderAdvertisingParameters, GattWriteOption, GattWriteRequestedEventArgs,
};
use windows::Foundation::{Deferral, TypedEventHandler};
use windows::Storage::Streams::DataReader;
use windows::core::Result;

use crate::config::SharedAppState;
use crate::dispatch;
use crate::i18n::{self, AppLanguage, BleAdvertiseStatusText};
use crate::protocol::{GESTURE_CHARACTERISTIC_UUID, SERVICE_UUID, SERVICE_UUID_STRING};

struct BleWriteJob {
    args: GattWriteRequestedEventArgs,
    deferral: Deferral,
    state: SharedAppState,
}

pub async fn run(state: SharedAppState) -> Result<()> {
    set_ble_status(&state, i18n::ble_creating(language(&state)), None);

    let provider_result = GattServiceProvider::CreateAsync(SERVICE_UUID)?.await?;
    let provider = provider_result.ServiceProvider()?;
    let service = provider.Service()?;

    let characteristic_parameters = GattLocalCharacteristicParameters::new()?;
    characteristic_parameters.SetCharacteristicProperties(
        GattCharacteristicProperties::Write | GattCharacteristicProperties::WriteWithoutResponse,
    )?;
    characteristic_parameters.SetWriteProtectionLevel(GattProtectionLevel::Plain)?;
    characteristic_parameters.SetUserDescription(&"TouchBridge Compact Input".into())?;

    let characteristic_result = service
        .CreateCharacteristicAsync(GESTURE_CHARACTERISTIC_UUID, &characteristic_parameters)?
        .await?;
    let characteristic = characteristic_from_result(characteristic_result)?;

    register_write_handler(&characteristic, state.clone())?;
    register_advertisement_handler(&provider, state.clone())?;

    let advertising_parameters = GattServiceProviderAdvertisingParameters::new()?;
    advertising_parameters.SetIsConnectable(true)?;
    advertising_parameters.SetIsDiscoverable(true)?;
    provider.StartAdvertisingWithParameters(&advertising_parameters)?;

    let status = i18n::ble_advertising(language(&state), SERVICE_UUID_STRING);
    println!("{status}");
    set_ble_status(&state, status, None);

    // Keep WinRT BLE objects alive for the lifetime of the agent.
    loop {
        thread::park();
    }
}

fn characteristic_from_result(
    result: GattLocalCharacteristicResult,
) -> Result<GattLocalCharacteristic> {
    result.Characteristic()
}

fn register_write_handler(
    characteristic: &GattLocalCharacteristic,
    state: SharedAppState,
) -> Result<()> {
    let write_sender = start_write_worker();
    let handler = TypedEventHandler::<GattLocalCharacteristic, GattWriteRequestedEventArgs>::new(
        move |_sender, args| {
            let Some(args) = args.cloned() else {
                return Ok(());
            };

            let deferral = args.GetDeferral()?;
            let state = state.clone();

            if let Err(err) = write_sender.send(BleWriteJob {
                args,
                deferral,
                state,
            }) {
                let job = err.0;
                eprintln!("BLE write queue failed");
                if let Err(err) = job.deferral.Complete() {
                    eprintln!("BLE write deferral completion failed: {err}");
                }
            }

            Ok(())
        },
    );

    characteristic.WriteRequested(&handler)?;
    Ok(())
}

fn start_write_worker() -> Sender<BleWriteJob> {
    let (sender, receiver) = mpsc::channel::<BleWriteJob>();

    thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create BLE write runtime");

        while let Ok(job) = receiver.recv() {
            if let Err(err) = handle_write(&runtime, job.args, job.state) {
                eprintln!("BLE write failed: {err}");
            }

            if let Err(err) = job.deferral.Complete() {
                eprintln!("BLE write deferral completion failed: {err}");
            }
        }
    });

    sender
}

fn handle_write(
    runtime: &tokio::runtime::Runtime,
    args: GattWriteRequestedEventArgs,
    state: SharedAppState,
) -> Result<()> {
    let request = runtime.block_on(args.GetRequestAsync()?.into_future())?;
    let write_option = request.Option()?;
    let value = request.Value()?;
    let raw = buffer_to_string(value)?;

    let _ = dispatch::handle_raw_message(&raw, "ble", &state);

    if write_option == GattWriteOption::WriteWithResponse {
        if let Err(err) = request.Respond() {
            eprintln!("BLE write response failed: {err}");
        }
    }

    Ok(())
}

fn buffer_to_string(buffer: windows::Storage::Streams::IBuffer) -> Result<String> {
    let reader = DataReader::FromBuffer(&buffer)?;
    let length = reader.UnconsumedBufferLength()?;
    let mut bytes = vec![0; length as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).trim().to_string())
}

fn register_advertisement_handler(
    provider: &GattServiceProvider,
    state: SharedAppState,
) -> Result<()> {
    let handler = TypedEventHandler::<
        GattServiceProvider,
        GattServiceProviderAdvertisementStatusChangedEventArgs,
    >::new(move |_sender, args| {
        let Some(args) = args.cloned() else {
            return Ok(());
        };

        let status = args.Status()?;
        let error = args.Error()?;
        let language = language(&state);
        let message = format!(
            "{} ({error:?})",
            i18n::ble_advertising_status(language, status_text(status, language)),
        );
        println!("{message}");
        set_ble_status(&state, message, None);
        Ok(())
    });

    provider.AdvertisementStatusChanged(&handler)?;
    Ok(())
}

fn status_text(
    status: GattServiceProviderAdvertisementStatus,
    language: AppLanguage,
) -> &'static str {
    let status = match status {
        GattServiceProviderAdvertisementStatus::Created => BleAdvertiseStatusText::Created,
        GattServiceProviderAdvertisementStatus::Stopped => BleAdvertiseStatusText::Stopped,
        GattServiceProviderAdvertisementStatus::Started => BleAdvertiseStatusText::Started,
        GattServiceProviderAdvertisementStatus::Aborted => BleAdvertiseStatusText::Aborted,
        GattServiceProviderAdvertisementStatus::StartedWithoutAllAdvertisementData => {
            BleAdvertiseStatusText::StartedPartial
        }
        _ => BleAdvertiseStatusText::Unknown,
    };
    i18n::ble_status_text(language, status)
}

fn set_ble_status(state: &SharedAppState, status: impl Into<String>, error: Option<String>) {
    let mut state = state.write().expect("app state poisoned");
    state.ble_status = status.into();
    state.ble_error = error;
}

fn language(state: &SharedAppState) -> AppLanguage {
    state.read().expect("app state poisoned").config.language
}
