use std::net::Ipv4Addr;
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde_json::Value;

use crate::config::SharedAppState;
use crate::i18n::AppLanguage;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const USB_TETHER_METRIC: u32 = 9000;
const CHECK_INTERVAL: Duration = Duration::from_secs(10);

const QUERY_ADAPTERS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$ifaces = @(Get-NetIPInterface -AddressFamily IPv4 | Where-Object { $_.ConnectionState -eq 'Connected' })
$configs = @(Get-NetIPConfiguration)
$adapters = @(Get-NetAdapter -ErrorAction SilentlyContinue)
$profiles = @(Get-NetConnectionProfile -ErrorAction SilentlyContinue)
$items = @()

foreach ($iface in $ifaces) {
    $config = $configs | Where-Object { $_.InterfaceIndex -eq $iface.InterfaceIndex } | Select-Object -First 1
    $adapter = $adapters | Where-Object { $_.ifIndex -eq $iface.InterfaceIndex } | Select-Object -First 1
    $profile = $profiles | Where-Object { $_.InterfaceIndex -eq $iface.InterfaceIndex } | Select-Object -First 1

    $items += [PSCustomObject]@{
        InterfaceAlias = [string]$iface.InterfaceAlias
        InterfaceIndex = [int]$iface.InterfaceIndex
        InterfaceMetric = [int]$iface.InterfaceMetric
        ConnectionState = [string]$iface.ConnectionState
        Description = if ($adapter) { [string]$adapter.InterfaceDescription } else { "" }
        Status = if ($adapter) { [string]$adapter.Status } else { "" }
        Gateway = @($config.IPv4DefaultGateway | ForEach-Object { [string]$_.NextHop })
        IpAddresses = @($config.IPv4Address | ForEach-Object { [string]$_.IPAddress })
        IPv4Connectivity = if ($profile) { [string]$profile.IPv4Connectivity } else { "" }
    }
}

[PSCustomObject]@{ Adapters = $items } | ConvertTo-Json -Compress -Depth 5
"#;

pub fn run(state: SharedAppState) {
    loop {
        update_route_guard_state(&state);
        thread::sleep(CHECK_INTERVAL);
    }
}

fn update_route_guard_state(state: &SharedAppState) {
    let (enabled, language) = {
        let state = state.read().expect("app state poisoned");
        (
            state.config.usb_tether_route_guard_enabled,
            state.config.language,
        )
    };

    let report = match inspect_and_protect(enabled) {
        Ok(report) => report,
        Err(error) => RouteGuardReport::query_failed(error),
    };

    println!("Windows route guard: {}", report.log_summary());

    let mut state = state.write().expect("app state poisoned");
    state.internet_route_status = report.status_text(language);
    state.internet_route_error = report.error_text(language);
}

fn inspect_and_protect(enabled: bool) -> std::io::Result<RouteGuardReport> {
    let mut adapters = query_adapters()?;
    let usb_candidates = usb_tether_candidates(&adapters);
    let usb_aliases = usb_candidates
        .iter()
        .map(|adapter| adapter.interface_alias.clone())
        .collect::<Vec<_>>();
    let existing_internet = existing_internet_adapters(&adapters);
    let existing_internet_aliases = existing_internet
        .iter()
        .map(|adapter| adapter.interface_alias.clone())
        .collect::<Vec<_>>();
    let has_existing_internet = !existing_internet.is_empty();
    let best_existing_metric = existing_internet
        .iter()
        .filter_map(|adapter| adapter.interface_metric)
        .min();

    log_adapters(&adapters);

    if usb_candidates.is_empty() {
        return Ok(RouteGuardReport::NoUsbTether {
            adapter_count: adapters.len(),
        });
    }

    if !enabled {
        return Ok(RouteGuardReport::Disabled { usb_aliases });
    }

    let mut adjusted_aliases = Vec::new();
    let mut failed = Vec::new();
    let adjustment_targets = usb_candidates
        .iter()
        .filter(|adapter| adapter.safe_to_adjust_usb_tether_metric())
        .map(|adapter| {
            (
                adapter.interface_index,
                adapter.interface_alias.clone(),
                adapter.interface_metric,
            )
        })
        .collect::<Vec<_>>();

    for (interface_index, interface_alias, interface_metric) in adjustment_targets {
        if interface_metric.unwrap_or(u32::MAX) >= USB_TETHER_METRIC {
            continue;
        }

        match set_interface_metric(interface_index, USB_TETHER_METRIC) {
            Ok(()) => {
                adjusted_aliases.push(interface_alias.clone());
                if let Some(adapter) = adapters
                    .iter_mut()
                    .find(|adapter| adapter.interface_index == interface_index)
                {
                    adapter.interface_metric = Some(USB_TETHER_METRIC);
                }
            }
            Err(error) => failed.push(MetricAdjustmentFailure {
                alias: interface_alias,
                error: error.to_string(),
            }),
        }
    }

    if !failed.is_empty() {
        return Ok(RouteGuardReport::MetricAdjustmentFailed {
            usb_aliases,
            failures: failed,
        });
    }

    if !has_existing_internet {
        return Ok(RouteGuardReport::PhoneDataRisk {
            usb_aliases,
            adjusted_aliases,
        });
    }

    let risky_usb_aliases: Vec<String> = usb_tether_candidates(&adapters)
        .iter()
        .filter(|adapter| {
            let Some(usb_metric) = adapter.interface_metric else {
                return true;
            };
            best_existing_metric
                .map(|internet_metric| usb_metric <= internet_metric)
                .unwrap_or(true)
        })
        .map(|adapter| adapter.interface_alias.clone())
        .collect();

    if !risky_usb_aliases.is_empty() {
        return Ok(RouteGuardReport::PhoneDataRisk {
            usb_aliases: risky_usb_aliases,
            adjusted_aliases,
        });
    }

    Ok(RouteGuardReport::Protected {
        internet_aliases: existing_internet_aliases,
        usb_aliases,
        adjusted_aliases,
    })
}

fn query_adapters() -> std::io::Result<Vec<NetworkAdapter>> {
    let output = powershell_command(QUERY_ADAPTERS_SCRIPT).output()?;

    if !output.status.success() {
        return Err(command_error("Get-NetIPInterface", &output.stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value = serde_json::from_str::<Value>(&stdout).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("PowerShell adapter JSON parse failed: {error}; raw={stdout}"),
        )
    })?;

    Ok(parse_adapter_inventory(&value))
}

fn set_interface_metric(interface_index: u32, metric: u32) -> std::io::Result<()> {
    let script = format!(
        "Set-NetIPInterface -InterfaceIndex {interface_index} -AddressFamily IPv4 -InterfaceMetric {metric} -ErrorAction Stop"
    );
    let output = powershell_command(&script).output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("Set-NetIPInterface", &output.stderr))
    }
}

fn powershell_command(script: &str) -> Command {
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

fn command_error(command: &str, stderr: &[u8]) -> std::io::Error {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    let message = if stderr.is_empty() {
        format!("{command} failed")
    } else {
        format!("{command} failed: {stderr}")
    };

    std::io::Error::new(std::io::ErrorKind::PermissionDenied, message)
}

fn parse_adapter_inventory(value: &Value) -> Vec<NetworkAdapter> {
    let adapters = match value.get("Adapters") {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::Object(_)) => vec![value["Adapters"].clone()],
        _ => Vec::new(),
    };

    adapters
        .iter()
        .filter_map(parse_network_adapter)
        .collect::<Vec<_>>()
}

fn parse_network_adapter(value: &Value) -> Option<NetworkAdapter> {
    let interface_alias = string_field(value, "InterfaceAlias")?;
    let interface_index = u32_field(value, "InterfaceIndex")?;

    Some(NetworkAdapter {
        interface_alias,
        interface_index,
        interface_metric: u32_field(value, "InterfaceMetric"),
        description: string_field(value, "Description").unwrap_or_default(),
        connection_state: string_field(value, "ConnectionState").unwrap_or_default(),
        status: string_field(value, "Status").unwrap_or_default(),
        gateways: string_vec_field(value, "Gateway"),
        ip_addresses: string_vec_field(value, "IpAddresses"),
        ipv4_connectivity: string_field(value, "IPv4Connectivity").unwrap_or_default(),
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key)?.as_str().map(str::trim).and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    })
}

fn u32_field(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str()?.parse::<u64>().ok())
        })
        .and_then(|value| u32::try_from(value).ok())
}

fn string_vec_field(value: &Value, key: &str) -> Vec<String> {
    match value.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::trim))
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        Some(Value::String(item)) if !item.trim().is_empty() => vec![item.trim().to_string()],
        _ => Vec::new(),
    }
}

fn usb_tether_candidates(adapters: &[NetworkAdapter]) -> Vec<&NetworkAdapter> {
    adapters
        .iter()
        .filter(|adapter| adapter.is_usb_tether_candidate())
        .collect()
}

fn existing_internet_adapters(adapters: &[NetworkAdapter]) -> Vec<&NetworkAdapter> {
    adapters
        .iter()
        .filter(|adapter| adapter.is_existing_internet_adapter())
        .collect()
}

fn log_adapters(adapters: &[NetworkAdapter]) {
    if adapters.is_empty() {
        println!("Windows route guard adapters: none");
        return;
    }

    let summary = adapters
        .iter()
        .map(NetworkAdapter::diagnostic_summary)
        .collect::<Vec<_>>()
        .join("; ");
    println!("Windows route guard adapters: {summary}");
}

#[derive(Clone, Debug)]
struct NetworkAdapter {
    interface_alias: String,
    interface_index: u32,
    interface_metric: Option<u32>,
    description: String,
    connection_state: String,
    status: String,
    gateways: Vec<String>,
    ip_addresses: Vec<String>,
    ipv4_connectivity: String,
}

impl NetworkAdapter {
    fn has_default_gateway(&self) -> bool {
        !self.gateways.is_empty()
    }

    fn identity_text(&self) -> String {
        format!("{} {}", self.interface_alias, self.description).to_ascii_lowercase()
    }

    fn is_usb_tether_candidate(&self) -> bool {
        self.has_strong_usb_tether_signal()
            || self.has_usb_tether_ip()
            || self.identity_text().contains("usb")
    }

    fn safe_to_adjust_usb_tether_metric(&self) -> bool {
        self.has_default_gateway()
            && (self.has_strong_usb_tether_signal() || self.has_usb_tether_ip())
    }

    fn has_strong_usb_tether_signal(&self) -> bool {
        let text = self.identity_text();
        [
            "android",
            "rndis",
            "remote ndis",
            "mobile",
            "tether",
            "samsung",
            "pixel",
        ]
        .iter()
        .any(|needle| text.contains(needle))
    }

    fn has_usb_tether_ip(&self) -> bool {
        self.ip_addresses
            .iter()
            .filter_map(|ip| ip.parse::<Ipv4Addr>().ok())
            .any(is_usb_tether_ipv4)
    }

    fn is_existing_internet_adapter(&self) -> bool {
        if !self.has_default_gateway() || self.is_usb_tether_candidate() {
            return false;
        }

        let text = self.identity_text();
        let looks_like_wifi_or_ethernet = [
            "wi-fi", "wifi", "wireless", "wlan", "ethernet", "gigabit", "lan",
        ]
        .iter()
        .any(|needle| text.contains(needle));

        looks_like_wifi_or_ethernet || self.ipv4_connectivity.eq_ignore_ascii_case("internet")
    }

    fn diagnostic_summary(&self) -> String {
        format!(
            "alias={} index={} metric={} gateway={} ip={} desc={} state={} status={} ipv4={}",
            self.interface_alias,
            self.interface_index,
            self.interface_metric
                .map(|metric| metric.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.gateways.join("|"),
            self.ip_addresses.join("|"),
            self.description,
            self.connection_state,
            self.status,
            self.ipv4_connectivity,
        )
    }
}

fn is_usb_tether_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    matches!(
        octets,
        [192, 168, 42, _] | [192, 168, 43, _] | [192, 168, 44, _] | [172, 20, _, _]
    )
}

#[derive(Clone, Debug)]
struct MetricAdjustmentFailure {
    alias: String,
    error: String,
}

#[derive(Clone, Debug)]
enum RouteGuardReport {
    NoUsbTether {
        adapter_count: usize,
    },
    Disabled {
        usb_aliases: Vec<String>,
    },
    Protected {
        internet_aliases: Vec<String>,
        usb_aliases: Vec<String>,
        adjusted_aliases: Vec<String>,
    },
    PhoneDataRisk {
        usb_aliases: Vec<String>,
        adjusted_aliases: Vec<String>,
    },
    MetricAdjustmentFailed {
        usb_aliases: Vec<String>,
        failures: Vec<MetricAdjustmentFailure>,
    },
    QueryFailed {
        error: String,
    },
}

impl RouteGuardReport {
    fn query_failed(error: std::io::Error) -> Self {
        Self::QueryFailed {
            error: error.to_string(),
        }
    }

    fn status_text(&self, language: AppLanguage) -> String {
        match language {
            AppLanguage::English => self.english_status_text(),
            AppLanguage::Korean => self.korean_status_text(),
        }
    }

    fn error_text(&self, language: AppLanguage) -> Option<String> {
        match language {
            AppLanguage::English => self.english_error_text(),
            AppLanguage::Korean => self.korean_error_text(),
        }
    }

    fn log_summary(&self) -> String {
        match self {
            Self::NoUsbTether { adapter_count } => {
                format!("no USB tethering candidate detected; adapters={adapter_count}")
            }
            Self::Disabled { usb_aliases } => {
                format!("disabled; usb candidates={}", usb_aliases.join(", "))
            }
            Self::Protected {
                internet_aliases,
                usb_aliases,
                adjusted_aliases,
            } => format!(
                "protected; internet={}; usb={}; adjusted={}",
                internet_aliases.join(", "),
                usb_aliases.join(", "),
                adjusted_aliases.join(", "),
            ),
            Self::PhoneDataRisk {
                usb_aliases,
                adjusted_aliases,
            } => format!(
                "phone data risk; usb={}; adjusted={}",
                usb_aliases.join(", "),
                adjusted_aliases.join(", "),
            ),
            Self::MetricAdjustmentFailed {
                usb_aliases,
                failures,
            } => format!(
                "metric adjustment failed; usb={}; failures={}",
                usb_aliases.join(", "),
                failures
                    .iter()
                    .map(|failure| format!("{}: {}", failure.alias, failure.error))
                    .collect::<Vec<_>>()
                    .join(" | "),
            ),
            Self::QueryFailed { error } => format!("query failed: {error}"),
        }
    }

    fn english_status_text(&self) -> String {
        match self {
            Self::NoUsbTether { .. } => {
                "USB tethering: not detected; internet route unchanged".to_string()
            }
            Self::Disabled { usb_aliases } => format!(
                "USB tethering route guard is OFF; metric unchanged for {}",
                join_aliases(usb_aliases)
            ),
            Self::Protected {
                internet_aliases,
                usb_aliases,
                adjusted_aliases,
            } => {
                let adjusted = adjusted_suffix(adjusted_aliases);
                format!(
                    "Internet route: keeping existing Wi-Fi/Ethernet ({}) ; USB tethering: remote control only ({}){}",
                    join_aliases(internet_aliases),
                    join_aliases(usb_aliases),
                    adjusted,
                )
            }
            Self::PhoneDataRisk {
                usb_aliases,
                adjusted_aliases,
            } => {
                let adjusted = adjusted_suffix(adjusted_aliases);
                format!(
                    "Warning: Windows internet may use phone data through {}{}",
                    join_aliases(usb_aliases),
                    adjusted,
                )
            }
            Self::MetricAdjustmentFailed { usb_aliases, .. } => format!(
                "Warning: USB tethering metric adjustment failed for {}",
                join_aliases(usb_aliases)
            ),
            Self::QueryFailed { .. } => {
                "USB tethering route guard could not inspect Windows adapters".to_string()
            }
        }
    }

    fn korean_status_text(&self) -> String {
        match self {
            Self::NoUsbTether { .. } => {
                "USB 테더링: 감지되지 않음; 인터넷 경로 변경 없음".to_string()
            }
            Self::Disabled { usb_aliases } => format!(
                "USB 테더링 라우팅 보호 OFF; {} metric 변경 안 함",
                join_aliases(usb_aliases)
            ),
            Self::Protected {
                internet_aliases,
                usb_aliases,
                adjusted_aliases,
            } => {
                let adjusted = adjusted_suffix_ko(adjusted_aliases);
                format!(
                    "인터넷 경로: 기존 Wi-Fi/Ethernet 유지 중({}); USB 테더링: 원격 조작 전용({}){}",
                    join_aliases(internet_aliases),
                    join_aliases(usb_aliases),
                    adjusted,
                )
            }
            Self::PhoneDataRisk {
                usb_aliases,
                adjusted_aliases,
            } => {
                let adjusted = adjusted_suffix_ko(adjusted_aliases);
                format!(
                    "경고: 현재 PC 인터넷이 휴대폰 데이터를 사용할 수 있음({}){}",
                    join_aliases(usb_aliases),
                    adjusted,
                )
            }
            Self::MetricAdjustmentFailed { usb_aliases, .. } => format!(
                "경고: USB 테더링 metric 조정 실패({})",
                join_aliases(usb_aliases)
            ),
            Self::QueryFailed { .. } => {
                "USB 테더링 라우팅 보호가 Windows 어댑터를 검사하지 못함".to_string()
            }
        }
    }

    fn english_error_text(&self) -> Option<String> {
        match self {
            Self::MetricAdjustmentFailed { failures, .. } => Some(format!(
                "Administrator permission may be required: {}",
                failure_summary(failures)
            )),
            Self::QueryFailed { error } => Some(format!("Route guard inspection failed: {error}")),
            Self::PhoneDataRisk { .. } => {
                Some("No existing Wi-Fi/Ethernet internet route was confirmed.".to_string())
            }
            Self::Disabled { .. } => {
                Some("Protection is disabled; Windows may prefer phone data.".to_string())
            }
            _ => None,
        }
    }

    fn korean_error_text(&self) -> Option<String> {
        match self {
            Self::MetricAdjustmentFailed { failures, .. } => Some(format!(
                "관리자 권한 필요: USB 테더링 metric 조정 실패: {}",
                failure_summary(failures)
            )),
            Self::QueryFailed { error } => Some(format!("라우팅 보호 검사 실패: {error}")),
            Self::PhoneDataRisk { .. } => {
                Some("기존 Wi-Fi/Ethernet 인터넷 경로를 확인하지 못했습니다.".to_string())
            }
            Self::Disabled { .. } => Some(
                "보호 기능이 꺼져 있어 Windows가 휴대폰 데이터를 우선 사용할 수 있습니다."
                    .to_string(),
            ),
            _ => None,
        }
    }
}

fn join_aliases(aliases: &[String]) -> String {
    if aliases.is_empty() {
        "none".to_string()
    } else {
        aliases.join(", ")
    }
}

fn adjusted_suffix(aliases: &[String]) -> String {
    if aliases.is_empty() {
        String::new()
    } else {
        format!(
            "; metric set to {USB_TETHER_METRIC} for {}",
            join_aliases(aliases)
        )
    }
}

fn adjusted_suffix_ko(aliases: &[String]) -> String {
    if aliases.is_empty() {
        String::new()
    } else {
        format!(
            "; {} metric을 {USB_TETHER_METRIC}으로 조정함",
            join_aliases(aliases)
        )
    }
}

fn failure_summary(failures: &[MetricAdjustmentFailure]) -> String {
    failures
        .iter()
        .map(|failure| format!("{}: {}", failure.alias, failure.error))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_keyword_is_safe_usb_tether_candidate() {
        let adapter = adapter(
            "Ethernet 3",
            "Remote NDIS based Internet Sharing Device",
            Some(25),
            &["192.168.42.129"],
            &["192.168.42.1"],
        );

        assert!(adapter.is_usb_tether_candidate());
        assert!(adapter.safe_to_adjust_usb_tether_metric());
    }

    #[test]
    fn tether_subnet_is_safe_usb_tether_candidate() {
        let adapter = adapter(
            "Network Adapter",
            "Unknown",
            Some(35),
            &["172.20.10.2"],
            &["172.20.10.1"],
        );

        assert!(adapter.is_usb_tether_candidate());
        assert!(adapter.safe_to_adjust_usb_tether_metric());
    }

    #[test]
    fn generic_usb_without_tether_signal_is_candidate_but_not_adjusted() {
        let adapter = adapter(
            "USB Ethernet",
            "Generic USB Ethernet Adapter",
            Some(10),
            &["10.0.0.5"],
            &["10.0.0.1"],
        );

        assert!(adapter.is_usb_tether_candidate());
        assert!(!adapter.safe_to_adjust_usb_tether_metric());
    }

    #[test]
    fn wifi_gateway_is_existing_internet_adapter() {
        let adapter = adapter(
            "Wi-Fi",
            "Intel Wireless",
            Some(20),
            &["192.168.0.12"],
            &["192.168.0.1"],
        );

        assert!(adapter.is_existing_internet_adapter());
    }

    #[test]
    fn parses_powershell_adapter_inventory() {
        let raw = serde_json::json!({
            "Adapters": {
                "InterfaceAlias": "Ethernet 4",
                "InterfaceIndex": 12,
                "InterfaceMetric": 25,
                "ConnectionState": "Connected",
                "Description": "Remote NDIS Compatible Device",
                "Status": "Up",
                "Gateway": ["192.168.42.1"],
                "IpAddresses": ["192.168.42.129"],
                "IPv4Connectivity": "LocalNetwork"
            }
        });

        let adapters = parse_adapter_inventory(&raw);

        assert_eq!(1, adapters.len());
        assert_eq!("Ethernet 4", adapters[0].interface_alias);
        assert_eq!(12, adapters[0].interface_index);
        assert_eq!(Some(25), adapters[0].interface_metric);
        assert!(adapters[0].is_usb_tether_candidate());
    }

    fn adapter(
        alias: &str,
        description: &str,
        metric: Option<u32>,
        ip_addresses: &[&str],
        gateways: &[&str],
    ) -> NetworkAdapter {
        NetworkAdapter {
            interface_alias: alias.to_string(),
            interface_index: 1,
            interface_metric: metric,
            description: description.to_string(),
            connection_state: "Connected".to_string(),
            status: "Up".to_string(),
            gateways: gateways.iter().map(|value| value.to_string()).collect(),
            ip_addresses: ip_addresses.iter().map(|value| value.to_string()).collect(),
            ipv4_connectivity: String::new(),
        }
    }
}
