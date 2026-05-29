package com.sharon77770.touchbridge

enum class DeviceType {
    Laptop,
    Desktop,
}

enum class DeviceOs {
    Windows,
}

enum class TransportType {
    Ble,
    Usb,
}

enum class ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Failed,
}

enum class UsbAttachmentState {
    CableDisconnected,
    CableConnected,
    AccessoryAvailable,
}

data class DeviceCapabilities(
    val supportsLowLatency: Boolean,
    val requiresPairing: Boolean,
    val requiresCable: Boolean,
)

data class Device(
    val id: String,
    val name: String,
    val type: DeviceType,
    val os: DeviceOs,
    val transport: TransportType,
    val paired: Boolean,
    val trusted: Boolean,
    val available: Boolean,
    val usbAttachmentState: UsbAttachmentState?,
    val lastConnectedAt: Long?,
    val connectionStatus: ConnectionStatus,
    val autoConnect: Boolean,
    val capabilities: DeviceCapabilities,
)

fun initialTouchBridgeDevices(
    usbAttachmentState: UsbAttachmentState = UsbAttachmentState.CableDisconnected,
): List<Device> {
    return listOf(
        Device(
            id = USB_DEVICE_ID,
            name = "USB TouchBridge Agent",
            type = DeviceType.Desktop,
            os = DeviceOs.Windows,
            transport = TransportType.Usb,
            paired = true,
            trusted = true,
            available = usbAttachmentState != UsbAttachmentState.CableDisconnected,
            usbAttachmentState = usbAttachmentState,
            lastConnectedAt = null,
            connectionStatus = ConnectionStatus.Disconnected,
            autoConnect = false,
            capabilities = DeviceCapabilities(
                supportsLowLatency = true,
                requiresPairing = false,
                requiresCable = true,
            ),
        ),
    )
}

const val USB_DEVICE_ID = "usb-touchbridge-agent"

fun bleDeviceFromScan(
    id: String,
    name: String,
    trusted: Boolean = false,
    lastConnectedAt: Long? = null,
): Device {
    return Device(
        id = id,
        name = name,
        type = DeviceType.Laptop,
        os = DeviceOs.Windows,
        transport = TransportType.Ble,
        paired = trusted,
        trusted = trusted,
        available = true,
        usbAttachmentState = null,
        lastConnectedAt = lastConnectedAt,
        connectionStatus = ConnectionStatus.Disconnected,
        autoConnect = false,
        capabilities = DeviceCapabilities(
            supportsLowLatency = false,
            requiresPairing = true,
            requiresCable = false,
        ),
    )
}
