package com.sharon77770.touchbridge

import android.content.Context
import java.io.IOException

interface ConnectionTransport {
    suspend fun connect(device: Device): Result<Unit>
    suspend fun sendRawMessage(raw: String): Result<ProtocolAck>
    suspend fun sendGestureEvent(event: GestureEvent): Result<ProtocolAck>
    fun disconnect()
}

class ConnectionManager(
    private val bleTransport: BleTransport,
    private val usbTransport: UsbTransport,
) {
    private var activeDevice: Device? = null
    private var activeTransport: ConnectionTransport? = null

    fun isUsbAvailable(): Boolean = usbTransport.isAccessoryAvailable()

    fun usbAttachmentState(): UsbAttachmentState = usbTransport.attachmentState()

    suspend fun scanBleDevices(): Result<List<Device>> {
        return bleTransport.scanDevices()
    }

    suspend fun connectToDevice(device: Device): Result<Unit> {
        val transport = transportFor(device.transport)
        return transport.connect(device).onSuccess {
            activeDevice = device
            activeTransport = transport
        }
    }

    suspend fun sendGestureEvent(event: GestureEvent): Result<ProtocolAck> {
        val transport = activeTransport
            ?: return Result.failure(IOException("No active TouchBridge connection"))

        return transport.sendGestureEvent(event)
    }

    suspend fun syncCustomButtons(buttons: List<CustomButton>): Result<ProtocolAck> {
        val transport = activeTransport
            ?: return Result.failure(IOException("No active TouchBridge connection"))
        val deviceId = activeDevice?.id ?: "android-touchbridge"

        return transport.sendRawMessage(customButtonSyncJson(deviceId, buttons))
    }

    suspend fun sendCustomButtonEvent(buttonId: String): Result<ProtocolAck> {
        val transport = activeTransport
            ?: return Result.failure(IOException("No active TouchBridge connection"))
        val deviceId = activeDevice?.id ?: "android-touchbridge"

        return transport.sendRawMessage(customButtonEventJson(deviceId, buttonId))
    }

    fun disconnect() {
        activeTransport?.disconnect()
        activeTransport = null
        activeDevice = null
    }

    private fun transportFor(type: TransportType): ConnectionTransport {
        return when (type) {
            TransportType.Ble -> bleTransport
            TransportType.Usb -> usbTransport
        }
    }

    companion object {
        fun create(context: Context): ConnectionManager {
            val appContext = context.applicationContext
            return ConnectionManager(
                bleTransport = BleTransport(TouchBridgeClient(appContext)),
                usbTransport = UsbTransport(appContext),
            )
        }
    }
}

class BleTransport(
    private val client: TouchBridgeClient,
) : ConnectionTransport {
    suspend fun scanDevices(): Result<List<Device>> {
        return client.discoverAgentDevice().map { listOf(it) }
    }

    override suspend fun connect(device: Device): Result<Unit> {
        return client.connectToAgent()
    }

    override suspend fun sendGestureEvent(event: GestureEvent): Result<ProtocolAck> {
        return sendRawMessage(event.toProtocolJson())
    }

    override suspend fun sendRawMessage(raw: String): Result<ProtocolAck> {
        return client.sendRawMessage(raw).map {
            ProtocolAck(ok = true, message = "executed")
        }
    }

    override fun disconnect() {
        client.close()
    }
}
