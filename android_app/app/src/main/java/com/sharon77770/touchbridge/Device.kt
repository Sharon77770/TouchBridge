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

private const val BLE_DEVICE_ID_PREFIX = "ble-"

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

fun bleDeviceId(address: String): String = "$BLE_DEVICE_ID_PREFIX$address"

fun bleAddressFromDeviceId(id: String): String? {
    return id
        .takeIf { it.startsWith(BLE_DEVICE_ID_PREFIX) }
        ?.removePrefix(BLE_DEVICE_ID_PREFIX)
        ?.takeIf { it.isNotBlank() }
}

fun Device.bleAddressOrNull(): String? {
    return if (transport == TransportType.Ble) {
        bleAddressFromDeviceId(id)
    } else {
        null
    }
}

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
