use std::future::IntoFuture;
use std::thread;

use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristicProperties, GattLocalCharacteristic, GattLocalCharacteristicParameters,
    GattLocalCharacteristicResult, GattProtectionLevel, GattServiceProvider,
    GattServiceProviderAdvertisementStatus, GattServiceProviderAdvertisementStatusChangedEventArgs,
    GattServiceProviderAdvertisingParameters, GattWriteRequestedEventArgs,
};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::DataReader;
use windows::core::Result;

use crate::config::SharedAppState;
use crate::dispatch;
use crate::i18n::{self, AppLanguage, BleAdvertiseStatusText};
use crate::protocol::{GESTURE_CHARACTERISTIC_UUID, SERVICE_UUID, SERVICE_UUID_STRING};

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
    characteristic_parameters.SetUserDescription(&"TouchBridge Gesture JSON".into())?;

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
    let handler = TypedEventHandler::<GattLocalCharacteristic, GattWriteRequestedEventArgs>::new(
        move |_sender, args| {
            let Some(args) = args.cloned() else {
                return Ok(());
            };

            let deferral = args.GetDeferral()?;
            let state = state.clone();

            thread::spawn(move || {
                if let Err(err) = handle_write(args, state) {
                    eprintln!("BLE write failed: {err}");
                }

                if let Err(err) = deferral.Complete() {
                    eprintln!("BLE write deferral completion failed: {err}");
                }
            });

            Ok(())
        },
    );

    characteristic.WriteRequested(&handler)?;
    Ok(())
}

fn handle_write(args: GattWriteRequestedEventArgs, state: SharedAppState) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().expect("failed to create BLE write runtime");
    let request = runtime.block_on(args.GetRequestAsync()?.into_future())?;
    let value = request.Value()?;
    let raw = buffer_to_string(value)?;

    println!("BLE request: {raw}");
    let _ = dispatch::handle_raw_message(&raw, "ble", &state);

    if let Err(err) = request.Respond() {
        eprintln!("BLE write response failed: {err}");
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
