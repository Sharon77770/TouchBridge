use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppLanguage {
    English,
    Korean,
}

impl AppLanguage {
    pub fn label(self) -> &'static str {
        match self {
            AppLanguage::English => "English",
            AppLanguage::Korean => "한국어",
        }
    }
}

impl Default for AppLanguage {
    fn default() -> Self {
        let locale = std::env::var("LANG")
            .or_else(|_| std::env::var("LC_ALL"))
            .unwrap_or_default()
            .to_ascii_lowercase();

        if locale.starts_with("ko") {
            AppLanguage::Korean
        } else {
            AppLanguage::English
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TextKey {
    SettingsInMemory,
    DefaultsRestoredInMemory,
    Save,
    ResetDefaults,
    Language,
    LastRequest,
    LastAction,
    ServiceUuid,
    GestureCharacteristicUuid,
    Gesture,
    Description,
    Test,
    Status,
    Run,
    Ok,
    CustomButtons,
    ButtonId,
    ButtonName,
    NoRequestYet,
    Idle,
    BleServiceNotStarted,
    UsbServiceNotStarted,
    InvalidRequest,
    InputFailed,
    NoAction,
    HotkeyAction,
    ActionType,
    ActionContent,
    PythonScriptAction,
    PowerShellScriptAction,
    OpenTouchBridge,
    Exit,
}

pub fn text(language: AppLanguage, key: TextKey) -> &'static str {
    match language {
        AppLanguage::English => match key {
            TextKey::SettingsInMemory => "Settings are in memory",
            TextKey::DefaultsRestoredInMemory => "Defaults restored in memory",
            TextKey::Save => "Save",
            TextKey::ResetDefaults => "Reset defaults",
            TextKey::Language => "Language",
            TextKey::LastRequest => "Last request",
            TextKey::LastAction => "Last action",
            TextKey::ServiceUuid => "Service UUID",
            TextKey::GestureCharacteristicUuid => "Gesture characteristic UUID",
            TextKey::Gesture => "Gesture",
            TextKey::Description => "Description",
            TextKey::Test => "Test",
            TextKey::Status => "Status",
            TextKey::Run => "Run",
            TextKey::Ok => "OK",
            TextKey::CustomButtons => "Custom buttons",
            TextKey::ButtonId => "Button ID",
            TextKey::ButtonName => "Button name",
            TextKey::NoRequestYet => "No request yet",
            TextKey::Idle => "Idle",
            TextKey::BleServiceNotStarted => "BLE service not started yet",
            TextKey::UsbServiceNotStarted => "USB service not started yet",
            TextKey::InvalidRequest => "Invalid request",
            TextKey::InputFailed => "Input failed",
            TextKey::NoAction => "No action",
            TextKey::HotkeyAction => "Hotkey",
            TextKey::ActionType => "Action type",
            TextKey::ActionContent => "Action",
            TextKey::PythonScriptAction => "Python code",
            TextKey::PowerShellScriptAction => "PowerShell script",
            TextKey::OpenTouchBridge => "Open TouchBridge",
            TextKey::Exit => "Exit",
        },
        AppLanguage::Korean => match key {
            TextKey::SettingsInMemory => "설정이 메모리에 적용됨",
            TextKey::DefaultsRestoredInMemory => "기본값을 메모리에 복원함",
            TextKey::Save => "저장",
            TextKey::ResetDefaults => "기본값 복원",
            TextKey::Language => "언어",
            TextKey::LastRequest => "마지막 요청",
            TextKey::LastAction => "마지막 동작",
            TextKey::ServiceUuid => "서비스 UUID",
            TextKey::GestureCharacteristicUuid => "제스처 특성 UUID",
            TextKey::Gesture => "제스처",
            TextKey::Description => "설명",
            TextKey::Test => "테스트",
            TextKey::Status => "상태",
            TextKey::Run => "실행",
            TextKey::Ok => "정상",
            TextKey::CustomButtons => "커스텀 버튼",
            TextKey::ButtonId => "버튼 ID",
            TextKey::ButtonName => "버튼 이름",
            TextKey::NoRequestYet => "아직 요청 없음",
            TextKey::Idle => "대기 중",
            TextKey::BleServiceNotStarted => "BLE 서비스가 아직 시작되지 않음",
            TextKey::UsbServiceNotStarted => "USB 서비스가 아직 시작되지 않음",
            TextKey::InvalidRequest => "잘못된 요청",
            TextKey::InputFailed => "입력 실행 실패",
            TextKey::NoAction => "동작 없음",
            TextKey::HotkeyAction => "단축키",
            TextKey::ActionType => "동작 타입",
            TextKey::ActionContent => "동작 내용",
            TextKey::PythonScriptAction => "Python 코드",
            TextKey::PowerShellScriptAction => "PowerShell 스크립트",
            TextKey::OpenTouchBridge => "TouchBridge 열기",
            TextKey::Exit => "종료",
        },
    }
}

pub fn saved_to(language: AppLanguage, path: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => format!("Saved to {path}"),
        AppLanguage::Korean => format!("{path}에 저장됨"),
    }
}

pub fn save_failed(language: AppLanguage, error: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => format!("Save failed: {error}"),
        AppLanguage::Korean => format!("저장 실패: {error}"),
    }
}

pub fn input_failed(language: AppLanguage, error: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => format!("Input failed: {error}"),
        AppLanguage::Korean => format!("입력 실행 실패: {error}"),
    }
}

pub fn invalid_request(language: AppLanguage, error: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => format!("invalid request: {error}"),
        AppLanguage::Korean => format!("잘못된 요청: {error}"),
    }
}

pub fn parse_error(language: AppLanguage, error: &str) -> String {
    if language == AppLanguage::English {
        return error.to_string();
    }

    if error == "empty key segment" {
        return "빈 키 구간이 있습니다".to_string();
    }

    if let Some(key) = error.strip_prefix("unsupported key: ") {
        return format!("지원하지 않는 키: {key}");
    }

    if let Some(key) = error.strip_prefix("duplicate key: ") {
        return format!("중복된 키: {key}");
    }

    if error == "python runtime was not found" {
        return "Python이 설치되어 있지 않거나 PATH에서 찾을 수 없습니다".to_string();
    }

    if error == "python code is empty" {
        return "Python 코드가 비어 있습니다".to_string();
    }

    if error == "powershell script is empty" {
        return "PowerShell 스크립트가 비어 있습니다".to_string();
    }

    error.to_string()
}

pub fn ble_creating(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "Creating BLE GATT service...",
        AppLanguage::Korean => "BLE GATT 서비스를 생성하는 중...",
    }
}

pub fn ble_advertising(language: AppLanguage, service_uuid: &str) -> String {
    match language {
        AppLanguage::English => format!("BLE advertising as TouchBridge service {service_uuid}"),
        AppLanguage::Korean => format!("TouchBridge 서비스 {service_uuid}로 BLE 광고 중"),
    }
}

pub fn ble_advertising_status(language: AppLanguage, status: &str) -> String {
    match language {
        AppLanguage::English => format!("BLE advertising status: {status}"),
        AppLanguage::Korean => format!("BLE 광고 상태: {status}"),
    }
}

pub fn ble_status_text(language: AppLanguage, status: BleAdvertiseStatusText) -> &'static str {
    match language {
        AppLanguage::English => match status {
            BleAdvertiseStatusText::Created => "created",
            BleAdvertiseStatusText::Stopped => "stopped",
            BleAdvertiseStatusText::Started => "started",
            BleAdvertiseStatusText::Aborted => "aborted",
            BleAdvertiseStatusText::StartedPartial => "started without all advertisement data",
            BleAdvertiseStatusText::Unknown => "unknown",
        },
        AppLanguage::Korean => match status {
            BleAdvertiseStatusText::Created => "생성됨",
            BleAdvertiseStatusText::Stopped => "중지됨",
            BleAdvertiseStatusText::Started => "시작됨",
            BleAdvertiseStatusText::Aborted => "중단됨",
            BleAdvertiseStatusText::StartedPartial => "일부 광고 데이터 없이 시작됨",
            BleAdvertiseStatusText::Unknown => "알 수 없음",
        },
    }
}

#[derive(Clone, Copy, Debug)]
pub enum BleAdvertiseStatusText {
    Created,
    Stopped,
    Started,
    Aborted,
    StartedPartial,
    Unknown,
}

pub fn ble_service_stopped(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "BLE service stopped",
        AppLanguage::Korean => "BLE 서비스 중지됨",
    }
}

pub fn usb_listener_started(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "USB AOA listener started. Waiting for TouchBridge USB accessory..."
        }
        AppLanguage::Korean => "USB AOA 리스너 시작됨. TouchBridge USB 액세서리를 기다리는 중...",
    }
}

pub fn usb_switching_to_accessory(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB cable detected. Requesting Android accessory mode...",
        AppLanguage::Korean => "USB 케이블 감지됨. Android 액세서리 모드를 요청하는 중...",
    }
}

pub fn usb_tcp_connected(language: AppLanguage, address: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => format!("USB cable network connected: {address}"),
        AppLanguage::Korean => format!("USB 케이블 네트워크 연결됨: {address}"),
    }
}

pub fn usb_device_opened(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB AOA device opened",
        AppLanguage::Korean => "USB AOA 기기 열림",
    }
}

pub fn usb_connection_failed(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB connection failed",
        AppLanguage::Korean => "USB 연결 실패",
    }
}

pub fn usb_waiting(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "USB waiting: connect cable and ensure Android is in accessory mode"
        }
        AppLanguage::Korean => "USB 대기 중: 케이블을 연결하고 Android 액세서리 모드를 확인하세요",
    }
}

pub fn usb_accessory_switch_failed(language: AppLanguage, error: impl std::fmt::Display) -> String {
    match language {
        AppLanguage::English => {
            format!("Could not request Android accessory mode: {error}")
        }
        AppLanguage::Korean => {
            format!("Android 액세서리 모드를 요청하지 못했습니다: {error}")
        }
    }
}

pub fn usb_scan_failed(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB scan failed",
        AppLanguage::Korean => "USB 검색 실패",
    }
}

pub fn usb_no_bulk_endpoints(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB AOA device has no bulk endpoints",
        AppLanguage::Korean => "USB AOA 기기에 bulk endpoint가 없습니다",
    }
}

pub fn usb_no_bulk_endpoints_detail(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => {
            "Windows can see the USB device, but no bulk endpoint is available. A WinUSB-compatible interface may be required."
        }
        AppLanguage::Korean => {
            "Windows에서 USB 기기는 보이지만 사용할 수 있는 bulk endpoint가 없습니다. WinUSB 호환 인터페이스가 필요할 수 있습니다."
        }
    }
}

pub fn usb_service_stopped(language: AppLanguage) -> &'static str {
    match language {
        AppLanguage::English => "USB service stopped",
        AppLanguage::Korean => "USB 서비스 중지됨",
    }
}
