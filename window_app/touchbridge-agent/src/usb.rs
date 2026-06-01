use std::collections::HashSet;
use std::future::IntoFuture;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time;
use windows::Devices::Enumeration::DeviceInformation;
use windows::Devices::Usb::{
    UsbBulkInPipe, UsbBulkOutPipe, UsbControlRecipient, UsbControlRequestType,
    UsbControlTransferType, UsbDevice, UsbSetupPacket, UsbTransferDirection,
};
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};
use windows::core::{Error, HRESULT, HSTRING, Result};

use crate::config::SharedAppState;
use crate::dispatch;
use crate::i18n::{self, AppLanguage};

const GOOGLE_USB_VENDOR_ID: u32 = 0x18D1;
const AOA_PRODUCT_IDS: [u32; 6] = [0x2D00, 0x2D01, 0x2D02, 0x2D03, 0x2D04, 0x2D05];
const READ_BUFFER_SIZE: u32 = 4096;
const AOA_GET_PROTOCOL: u8 = 51;
const AOA_SEND_STRING: u8 = 52;
const AOA_START_ACCESSORY: u8 = 53;
const AOA_MIN_PROTOCOL_VERSION: u16 = 1;
const AOA_CONTROL_TIMEOUT: Duration = Duration::from_secs(3);
const E_FAIL: HRESULT = HRESULT(0x80004005u32 as i32);
const USB_TCP_PORT: u16 = 47831;
const USB_TCP_BEACON_PORT: u16 = 47832;
const ANDROID_VENDOR_IDS: [u32; 16] = [
    0x04E8, // Samsung
    0x05C6, // Qualcomm reference / several Android devices
    0x0BB4, // HTC
    0x0B05, // ASUS
    0x0E8D, // MediaTek
    0x0FCE, // Sony
    0x1004, // LG
    0x12D1, // Huawei / Honor
    0x17EF, // Lenovo
    0x18D1, // Google
    0x19D2, // ZTE
    0x22B8, // Motorola
    0x22D9, // OPPO
    0x2717, // Xiaomi
    0x2A70, // OnePlus
    0x2D95, // Vivo
];
const AOA_STRINGS: [(u32, &str); 6] = [
    (0, "TouchBridge"),
    (1, "TouchBridge USB Agent"),
    (2, "TouchBridge gesture controller"),
    (3, "1.0"),
    (4, "https://touchbridge.local"),
    (5, "touchbridge-agent"),
];
static LOGGED_USB_OPEN_FAILURES: OnceLock<StdMutex<HashSet<(u32, u32, i32)>>> = OnceLock::new();

pub async fn run(state: SharedAppState) -> Result<()> {
    set_usb_status(&state, i18n::usb_listener_started(language(&state)), None);

    let tcp_connections = Arc::new(AtomicUsize::new(0));

    tokio::try_join!(
        run_aoa_loop(state.clone(), tcp_connections.clone()),
        run_tcp_listener(state.clone(), tcp_connections),
        run_tcp_beacon(),
    )?;

    Ok(())
}

async fn run_aoa_loop(state: SharedAppState, tcp_connections: Arc<AtomicUsize>) -> Result<()> {
    loop {
        if tcp_connections.load(Ordering::SeqCst) > 0 {
            time::sleep(Duration::from_secs(2)).await;
            continue;
        }

        match open_aoa_device().await {
            Ok(Some(device)) => {
                set_usb_status(&state, i18n::usb_device_opened(language(&state)), None);

                if let Err(err) = serve_device(device, state.clone()).await {
                    set_usb_status(
                        &state,
                        i18n::usb_connection_failed(language(&state)),
                        Some(err.to_string()),
                    );
                    eprintln!("USB connection failed: {err}");
                }
            }
            Ok(None) => match request_aoa_accessory_mode().await {
                Ok(true) => {
                    set_usb_status(
                        &state,
                        i18n::usb_switching_to_accessory(language(&state)),
                        None,
                    );
                    time::sleep(Duration::from_secs(2)).await;
                }
                Ok(false) => {
                    set_usb_status(&state, i18n::usb_waiting(language(&state)), None);
                    time::sleep(Duration::from_secs(2)).await;
                }
                Err(err) => {
                    set_usb_status(
                        &state,
                        i18n::usb_waiting(language(&state)),
                        Some(i18n::usb_accessory_switch_failed(language(&state), err)),
                    );
                    time::sleep(Duration::from_secs(2)).await;
                }
            },
            Err(err) => {
                set_usb_status(
                    &state,
                    i18n::usb_scan_failed(language(&state)),
                    Some(err.to_string()),
                );
                eprintln!("USB scan failed: {err}");
                time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn run_tcp_listener(state: SharedAppState, tcp_connections: Arc<AtomicUsize>) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", USB_TCP_PORT))
        .await
        .map_err(io_error)?;
    println!("USB cable network listener started on 0.0.0.0:{USB_TCP_PORT}");

    loop {
        let (stream, address) = listener.accept().await.map_err(io_error)?;
        let state = state.clone();
        let tcp_connections = tcp_connections.clone();

        tokio::spawn(async move {
            tcp_connections.fetch_add(1, Ordering::SeqCst);
            set_usb_status(
                &state,
                i18n::usb_tcp_connected(language(&state), address),
                None,
            );

            if let Err(err) = serve_tcp_stream(stream, state.clone()).await {
                set_usb_status(
                    &state,
                    i18n::usb_connection_failed(language(&state)),
                    Some(err.to_string()),
                );
                eprintln!("USB cable network connection failed: {err}");
            }

            tcp_connections.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

async fn run_tcp_beacon() -> Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", 0)).await.map_err(io_error)?;
    socket.set_broadcast(true).map_err(io_error)?;
    let payload = format!(
        r#"{{"type":"touchbridge_usb_tcp","name":"TouchBridge Agent","port":{USB_TCP_PORT}}}"#
    );
    println!("USB cable network beacon started on UDP port {USB_TCP_BEACON_PORT}");

    loop {
        let _ = socket
            .send_to(payload.as_bytes(), ("255.255.255.255", USB_TCP_BEACON_PORT))
            .await;
        time::sleep(Duration::from_secs(1)).await;
    }
}

async fn serve_tcp_stream(stream: TcpStream, state: SharedAppState) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await.map_err(io_error)? {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        println!("USB cable network request: {line}");
        let ack = dispatch::handle_raw_message(line, "usb", &state);
        let response = format!("{}\n", ack.to_compact());
        writer
            .write_all(response.as_bytes())
            .await
            .map_err(io_error)?;
        writer.flush().await.map_err(io_error)?;
    }

    Ok(())
}

async fn open_aoa_device() -> Result<Option<UsbDevice>> {
    for product_id in AOA_PRODUCT_IDS {
        let selector = UsbDevice::GetDeviceSelectorVidPidOnly(GOOGLE_USB_VENDOR_ID, product_id)?;
        let devices = DeviceInformation::FindAllAsyncAqsFilter(&selector)?
            .into_future()
            .await?;

        if devices.Size()? == 0 {
            continue;
        }

        for index in 0..devices.Size()? {
            let id = devices.GetAt(index)?.Id()?;
            match UsbDevice::FromIdAsync(&id)?.into_future().await {
                Ok(device) => return Ok(Some(device)),
                Err(err) => {
                    if should_log_usb_open_failure(GOOGLE_USB_VENDOR_ID, product_id, &err) {
                        eprintln!(
                            "AOA USB device open failed: {}",
                            describe_usb_open_error(&err)
                        );
                    }
                }
            }
        }
    }

    Ok(None)
}

async fn request_aoa_accessory_mode() -> Result<bool> {
    for candidate in android_usb_candidates().await? {
        match open_usb_device(candidate.vendor_id, candidate.product_id).await {
            Ok(Some(device)) => match start_aoa_accessory(&device).await {
                Ok(()) => {
                    println!(
                        "Requested AOA mode for USB device VID_{:04X}&PID_{:04X}",
                        candidate.vendor_id, candidate.product_id
                    );
                    return Ok(true);
                }
                Err(err) => {
                    eprintln!(
                        "AOA mode request failed for VID_{:04X}&PID_{:04X}: {err}",
                        candidate.vendor_id, candidate.product_id
                    );
                }
            },
            Ok(None) => {}
            Err(err) => {
                eprintln!(
                    "Could not open Android USB candidate VID_{:04X}&PID_{:04X}: {}",
                    candidate.vendor_id,
                    candidate.product_id,
                    describe_usb_open_error(&err),
                );
            }
        }
    }

    Ok(false)
}

async fn android_usb_candidates() -> Result<Vec<UsbVidPid>> {
    let devices = DeviceInformation::FindAllAsync()?.into_future().await?;
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for index in 0..devices.Size()? {
        let id = devices.GetAt(index)?.Id()?.to_string();
        let Some(candidate) = parse_vid_pid(&id) else {
            continue;
        };

        if !ANDROID_VENDOR_IDS.contains(&candidate.vendor_id) {
            continue;
        }

        if candidate.vendor_id == GOOGLE_USB_VENDOR_ID
            && AOA_PRODUCT_IDS.contains(&candidate.product_id)
        {
            continue;
        }

        if seen.insert((candidate.vendor_id, candidate.product_id)) {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

async fn open_usb_device(vendor_id: u32, product_id: u32) -> Result<Option<UsbDevice>> {
    let selector = UsbDevice::GetDeviceSelectorVidPidOnly(vendor_id, product_id)?;
    let devices = DeviceInformation::FindAllAsyncAqsFilter(&selector)?
        .into_future()
        .await?;

    if devices.Size()? == 0 {
        return Ok(None);
    }

    for index in 0..devices.Size()? {
        let id = devices.GetAt(index)?.Id()?;
        match UsbDevice::FromIdAsync(&id)?.into_future().await {
            Ok(device) => return Ok(Some(device)),
            Err(err) => {
                if should_log_usb_open_failure(vendor_id, product_id, &err) {
                    eprintln!(
                        "USB device candidate open failed: {}",
                        describe_usb_open_error(&err)
                    );
                }
            }
        }
    }

    Ok(None)
}

async fn start_aoa_accessory(device: &UsbDevice) -> Result<()> {
    let protocol_version = get_aoa_protocol_version(device).await?;

    if protocol_version < AOA_MIN_PROTOCOL_VERSION {
        return Err(Error::new(
            E_FAIL,
            format!("Android device does not support AOA protocol: {protocol_version}"),
        ));
    }

    for (index, value) in AOA_STRINGS {
        send_aoa_string(device, index, value).await?;
    }

    let setup = setup_packet(UsbTransferDirection::Out, AOA_START_ACCESSORY, 0, 0, 0)?;

    let operation = device.SendControlOutTransferAsyncNoBuffer(&setup)?;
    let _ = time::timeout(AOA_CONTROL_TIMEOUT, operation.into_future()).await;
    Ok(())
}

async fn get_aoa_protocol_version(device: &UsbDevice) -> Result<u16> {
    let setup = setup_packet(UsbTransferDirection::In, AOA_GET_PROTOCOL, 0, 0, 2)?;

    let buffer = time::timeout(
        AOA_CONTROL_TIMEOUT,
        device
            .SendControlInTransferAsyncNoBuffer(&setup)?
            .into_future(),
    )
    .await
    .map_err(|_| Error::from_thread())??;

    let bytes = read_buffer_bytes(&buffer, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

async fn send_aoa_string(device: &UsbDevice, index: u32, value: &str) -> Result<()> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(0);

    let setup = setup_packet(
        UsbTransferDirection::Out,
        AOA_SEND_STRING,
        0,
        index,
        bytes.len() as u32,
    )?;
    let buffer = bytes_to_buffer(&bytes)?;

    time::timeout(
        AOA_CONTROL_TIMEOUT,
        device
            .SendControlOutTransferAsync(&setup, &buffer)?
            .into_future(),
    )
    .await
    .map_err(|_| Error::from_thread())??;

    Ok(())
}

fn setup_packet(
    direction: UsbTransferDirection,
    request: u8,
    value: u32,
    index: u32,
    length: u32,
) -> Result<UsbSetupPacket> {
    let request_type = UsbControlRequestType::new()?;
    request_type.SetDirection(direction)?;
    request_type.SetControlTransferType(UsbControlTransferType::Vendor)?;
    request_type.SetRecipient(UsbControlRecipient::Device)?;

    let setup = UsbSetupPacket::new()?;
    setup.SetRequestType(&request_type)?;
    setup.SetRequest(request)?;
    setup.SetValue(value)?;
    setup.SetIndex(index)?;
    setup.SetLength(length)?;
    Ok(setup)
}

fn bytes_to_buffer(bytes: &[u8]) -> Result<IBuffer> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(bytes)?;
    writer.DetachBuffer()
}

fn read_buffer_bytes(buffer: &IBuffer, expected_len: usize) -> Result<Vec<u8>> {
    let reader = DataReader::FromBuffer(buffer)?;
    let mut bytes = vec![0u8; expected_len];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UsbVidPid {
    vendor_id: u32,
    product_id: u32,
}

fn parse_vid_pid(device_id: &str) -> Option<UsbVidPid> {
    let lower = device_id.to_ascii_lowercase();
    let vendor_id = parse_hex_after(&lower, "vid_")?;
    let product_id = parse_hex_after(&lower, "pid_")?;
    Some(UsbVidPid {
        vendor_id,
        product_id,
    })
}

fn parse_hex_after(value: &str, marker: &str) -> Option<u32> {
    let start = value.find(marker)? + marker.len();
    let hex = value.get(start..start + 4)?;

    if hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        None
    }
}

fn describe_usb_open_error(error: &Error) -> String {
    if error.code().0 == 0 {
        "Windows returned no accessible UsbDevice. The Android interface is probably bound to a system/OEM driver; using USB cable network fallback if available.".to_string()
    } else {
        error.to_string()
    }
}

fn should_log_usb_open_failure(vendor_id: u32, product_id: u32, error: &Error) -> bool {
    let logged = LOGGED_USB_OPEN_FAILURES.get_or_init(|| StdMutex::new(HashSet::new()));
    match logged.lock() {
        Ok(mut logged) => logged.insert((vendor_id, product_id, error.code().0)),
        Err(_) => true,
    }
}

async fn serve_device(device: UsbDevice, state: SharedAppState) -> Result<()> {
    let interface = device.DefaultInterface()?;
    let in_pipes = interface.BulkInPipes()?;
    let out_pipes = interface.BulkOutPipes()?;

    if in_pipes.Size()? == 0 || out_pipes.Size()? == 0 {
        let language = language(&state);
        set_usb_status(
            &state,
            i18n::usb_no_bulk_endpoints(language),
            Some(i18n::usb_no_bulk_endpoints_detail(language).to_string()),
        );
        return Ok(());
    }

    let in_pipe = in_pipes.GetAt(0)?;
    let out_pipe = out_pipes.GetAt(0)?;
    receive_loop(in_pipe, out_pipe, state).await
}

async fn receive_loop(
    in_pipe: UsbBulkInPipe,
    out_pipe: UsbBulkOutPipe,
    state: SharedAppState,
) -> Result<()> {
    let input_stream = in_pipe.InputStream()?;
    let output_stream = out_pipe.OutputStream()?;
    let reader = DataReader::CreateDataReader(&input_stream)?;
    let writer = DataWriter::CreateDataWriter(&output_stream)?;
    let mut pending = String::new();

    loop {
        let loaded = reader.LoadAsync(READ_BUFFER_SIZE)?.into_future().await?;
        if loaded == 0 {
            return Ok(());
        }

        let raw = reader.ReadString(loaded)?.to_string();
        pending.push_str(&raw);

        while let Some(newline_index) = pending.find('\n') {
            let line = pending[..newline_index].trim().to_string();
            pending = pending[newline_index + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            println!("USB request: {line}");
            let ack = dispatch::handle_raw_message(&line, "usb", &state);
            let response = format!("{}\n", ack.to_compact());
            writer.WriteString(&HSTRING::from(response))?;
            writer.StoreAsync()?.into_future().await?;
        }
    }
}

fn set_usb_status(state: &SharedAppState, status: impl Into<String>, error: Option<String>) {
    let mut state = state.write().expect("app state poisoned");
    state.usb_status = status.into();
    state.usb_error = error;
}

fn language(state: &SharedAppState) -> AppLanguage {
    state.read().expect("app state poisoned").config.language
}

fn io_error(error: std::io::Error) -> Error {
    Error::new(E_FAIL, error.to_string())
}
