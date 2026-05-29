package com.sharon77770.touchbridge

import android.Manifest
import android.annotation.SuppressLint
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattService
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.util.Log
import androidx.core.content.ContextCompat
import java.io.IOException
import java.util.UUID
import java.util.concurrent.atomic.AtomicLong
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

const val TOUCHBRIDGE_ANDROID_CLIENT_BUILD = "ble-gatt-2026-05-29-02"

private const val TAG = "TouchBridgeClient"
private const val SCAN_TIMEOUT_MS = 12_000L
private const val CONNECT_TIMEOUT_MS = 8_000L
private const val WRITE_TIMEOUT_MS = 4_000L
private const val SERVICE_DISCOVERY_DELAY_MS = 500L
private const val GATT_CACHE_RETRY_DELAY_MS = 650L

private val SERVICE_UUID: UUID = UUID.fromString("4f2b9d9c-39c1-4f35-8752-9f32fd325f61")
private val GESTURE_CHARACTERISTIC_UUID: UUID =
    UUID.fromString("8a7f1168-48af-4f0d-95ee-7d6701f34b46")

open class TouchBridgeClient(
    private val appContext: Context? = null,
) {
    private val requestIds = AtomicLong(0)
    private val sendMutex = Mutex()
    private var activeConnection: ActiveConnection? = null
    private var pendingWrite: CompletableDeferred<Int>? = null

    @SuppressLint("MissingPermission")
    open suspend fun discoverAgentDevice(): Result<Device> = sendMutex.withLock {
        withContext(Dispatchers.IO) {
            runCatching {
                val context = appContext ?: throw IllegalStateException("Android Context is required")
                ensurePermissions(context)

                val adapter = context
                    .getSystemService(BluetoothManager::class.java)
                    ?.adapter
                    ?: throw IOException("Bluetooth adapter is unavailable")

                if (!adapter.isEnabled) {
                    throw IOException("Bluetooth is disabled")
                }

                val scanResult = scanForAgent(adapter, requestIds.incrementAndGet())
                val address = scanResult.device.address
                val name = scanResult.device.name
                    ?: scanResult.scanRecord?.deviceName
                    ?: "TouchBridge BLE Agent"

                bleDeviceFromScan(
                    id = "ble-$address",
                    name = name,
                )
            }.onFailure { throwable ->
                Log.e(TAG, "BLE discovery failed", throwable)
            }
        }
    }

    @SuppressLint("MissingPermission")
    open suspend fun connectToAgent(): Result<Unit> = sendMutex.withLock {
        withContext(Dispatchers.IO) {
            runCatching {
                activeConnection ?: connect(requestIds.incrementAndGet()).also {
                    activeConnection = it
                }
                Unit
            }.onFailure { throwable ->
                closeConnection()
                Log.e(TAG, "BLE connect failed", throwable)
            }
        }
    }

    @SuppressLint("MissingPermission")
    open suspend fun sendGesture(gesture: TouchBridgeGesture): Result<Unit> {
        return sendGestureEvent(
            GestureEvent(
                deviceId = "ble-touchbridge-agent",
                gesture = gesture,
            ),
        )
    }

    @SuppressLint("MissingPermission")
    open suspend fun sendGestureEvent(event: GestureEvent): Result<Unit> = sendMutex.withLock {
        val requestId = requestIds.incrementAndGet()
        sendRawMessageLocked(event.toProtocolJson(), requestId)
    }

    @SuppressLint("MissingPermission")
    open suspend fun sendRawMessage(raw: String): Result<Unit> = sendMutex.withLock {
        val requestId = requestIds.incrementAndGet()
        sendRawMessageLocked(raw, requestId)
    }

    private suspend fun sendRawMessageLocked(raw: String, requestId: Long): Result<Unit> {
        val payload = raw

        return withContext(Dispatchers.IO) {
            runCatching {
                Log.d(TAG, "[$requestId] BLE write payload=$payload")

                try {
                    writePayload(payload.encodeToByteArray(), requestId)
                } catch (firstError: IOException) {
                    Log.w(TAG, "[$requestId] BLE write failed; reconnecting", firstError)
                    closeConnection()
                    writePayload(payload.encodeToByteArray(), requestId)
                }

                Log.d(TAG, "[$requestId] BLE sent")
                Unit
            }.onFailure { throwable ->
                closeConnection()
                Log.e(TAG, "[$requestId] BLE send failed", throwable)
            }
        }
    }

    fun close() {
        closeConnection()
    }

    private suspend fun writePayload(payload: ByteArray, requestId: Long) {
        val connection = activeConnection ?: connect(requestId).also {
            activeConnection = it
        }

        val writeResult = CompletableDeferred<Int>()
        pendingWrite = writeResult

        val started = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            connection.gatt.writeCharacteristic(
                connection.characteristic,
                payload,
                BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT,
            ) == BluetoothStatusCodes.SUCCESS
        } else {
            @Suppress("DEPRECATION")
            connection.characteristic.value = payload
            @Suppress("DEPRECATION")
            connection.gatt.writeCharacteristic(connection.characteristic)
        }

        if (!started) {
            pendingWrite = null
            throw IOException("BLE characteristic write did not start")
        }

        val status = withTimeout(WRITE_TIMEOUT_MS) {
            writeResult.await()
        }
        pendingWrite = null

        if (status != BluetoothGatt.GATT_SUCCESS) {
            throw IOException("BLE characteristic write failed with GATT status $status")
        }
    }

    @SuppressLint("MissingPermission")
    private suspend fun connect(requestId: Long): ActiveConnection {
        val context = appContext ?: throw IllegalStateException("Android Context is required")
        ensurePermissions(context)

        val adapter = context
            .getSystemService(BluetoothManager::class.java)
            ?.adapter
            ?: throw IOException("Bluetooth adapter is unavailable")

        if (!adapter.isEnabled) {
            throw IOException("Bluetooth is disabled")
        }

        val scanResult = scanForAgent(adapter, requestId)
        return try {
            connectGatt(context, scanResult.device, requestId)
        } catch (err: MissingGattSchemaException) {
            Log.w(TAG, "[$requestId] BLE GATT schema was incomplete; refreshing cache and retrying", err)
            delay(GATT_CACHE_RETRY_DELAY_MS)
            connectGatt(context, scanResult.device, requestId)
        }
    }

    @SuppressLint("MissingPermission")
    private suspend fun scanForAgent(
        adapter: BluetoothAdapter,
        requestId: Long,
    ): ScanResult {
        val scanner = adapter.bluetoothLeScanner
            ?: throw IOException("BLE scanner is unavailable")
        val result = CompletableDeferred<ScanResult>()
        val callback = object : ScanCallback() {
            override fun onScanResult(callbackType: Int, scanResult: ScanResult) {
                if (!result.isCompleted) {
                    Log.d(TAG, "[$requestId] found BLE agent ${scanResult.device.address}")
                    result.complete(scanResult)
                }
            }

            override fun onScanFailed(errorCode: Int) {
                if (!result.isCompleted) {
                    result.completeExceptionally(IOException("BLE scan failed with code $errorCode"))
                }
            }
        }

        val filter = ScanFilter.Builder()
            .setServiceUuid(ParcelUuid(SERVICE_UUID))
            .build()
        val settings = ScanSettings.Builder()
            .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
            .build()

        Log.d(TAG, "[$requestId] scanning for TouchBridge BLE service")
        scanner.startScan(listOf(filter), settings, callback)

        return try {
            withTimeout(SCAN_TIMEOUT_MS) {
                result.await()
            }
        } catch (err: TimeoutCancellationException) {
            throw IOException("TouchBridge BLE agent was not found", err)
        } finally {
            scanner.stopScan(callback)
        }
    }

    @SuppressLint("MissingPermission")
    private suspend fun connectGatt(
        context: Context,
        device: BluetoothDevice,
        requestId: Long,
    ): ActiveConnection {
        val result = CompletableDeferred<ActiveConnection>()
        val mainHandler = Handler(Looper.getMainLooper())

        fun startServiceDiscovery(gatt: BluetoothGatt) {
            if (result.isCompleted) return

            if (!gatt.discoverServices()) {
                if (!result.isCompleted) {
                    result.completeExceptionally(IOException("BLE service discovery did not start"))
                }
                closeGatt(gatt)
            }
        }

        val callback = object : BluetoothGattCallback() {
            override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
                when {
                    status != BluetoothGatt.GATT_SUCCESS -> {
                        if (!result.isCompleted) {
                            result.completeExceptionally(IOException("BLE connection failed with GATT status $status"))
                        }
                        closeGatt(gatt)
                    }
                    newState == BluetoothProfile.STATE_CONNECTED -> {
                        Log.d(
                            TAG,
                            "[$requestId] BLE connected; discovering services in ${SERVICE_DISCOVERY_DELAY_MS}ms",
                        )
                        mainHandler.postDelayed(
                            { startServiceDiscovery(gatt) },
                            SERVICE_DISCOVERY_DELAY_MS,
                        )
                    }
                    newState == BluetoothProfile.STATE_DISCONNECTED -> {
                        Log.d(TAG, "[$requestId] BLE disconnected")
                        if (!result.isCompleted) {
                            result.completeExceptionally(IOException("BLE disconnected before service discovery"))
                        }
                        if (activeConnection?.gatt == gatt) {
                            activeConnection = null
                        }
                        closeGatt(gatt)
                    }
                }
            }

            override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
                if (status != BluetoothGatt.GATT_SUCCESS) {
                    result.completeExceptionally(IOException("BLE service discovery failed with GATT status $status"))
                    closeGatt(gatt)
                    return
                }

                val discoveredServices = describeGattServices(gatt)
                Log.d(TAG, "[$requestId] discovered BLE services: $discoveredServices")

                val service: BluetoothGattService? = gatt.getService(SERVICE_UUID)
                val characteristic = service?.getCharacteristic(GESTURE_CHARACTERISTIC_UUID)

                if (service == null) {
                    refreshGattCache(gatt)
                    result.completeExceptionally(
                        MissingGattSchemaException(
                            "TouchBridge BLE service was not found. discovered=$discoveredServices",
                        ),
                    )
                    closeGatt(gatt)
                    return
                }

                if (characteristic == null) {
                    refreshGattCache(gatt)
                    result.completeExceptionally(
                        MissingGattSchemaException(
                            "TouchBridge BLE characteristic was not found. service=${service.uuid}, " +
                                "characteristics=${describeCharacteristics(service)}",
                        ),
                    )
                    closeGatt(gatt)
                    return
                }

                characteristic.writeType = BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
                Log.d(
                    TAG,
                    "[$requestId] TouchBridge BLE characteristic ready " +
                        "properties=${characteristicPropertiesText(characteristic.properties)}",
                )
                result.complete(ActiveConnection(gatt, characteristic))
            }

            override fun onCharacteristicWrite(
                gatt: BluetoothGatt,
                characteristic: BluetoothGattCharacteristic,
                status: Int,
            ) {
                pendingWrite?.complete(status)
            }
        }

        val gatt = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            device.connectGatt(context, false, callback, BluetoothDevice.TRANSPORT_LE)
        } else {
            @Suppress("DEPRECATION")
            device.connectGatt(context, false, callback)
        }

        return try {
            withTimeout(CONNECT_TIMEOUT_MS) {
                result.await()
            }
        } catch (err: TimeoutCancellationException) {
            closeGatt(gatt)
            throw IOException("BLE connection timed out", err)
        }
    }

    private fun ensurePermissions(context: Context) {
        val missing = requiredBluetoothPermissions().filter { permission ->
            ContextCompat.checkSelfPermission(context, permission) != PackageManager.PERMISSION_GRANTED
        }

        if (missing.isNotEmpty()) {
            throw SecurityException("Missing Bluetooth permission: ${missing.joinToString()}")
        }
    }

    @SuppressLint("MissingPermission")
    private fun closeConnection() {
        pendingWrite = null
        activeConnection?.gatt?.let(::closeGatt)
        activeConnection = null
    }

    @SuppressLint("MissingPermission")
    private fun closeGatt(gatt: BluetoothGatt?) {
        runCatching { gatt?.disconnect() }
        runCatching { gatt?.close() }
    }

    private fun refreshGattCache(gatt: BluetoothGatt) {
        val refreshed = runCatching {
            val refreshMethod = gatt.javaClass.getMethod("refresh")
            refreshMethod.invoke(gatt) as? Boolean ?: false
        }.onFailure { throwable ->
            Log.w(TAG, "BLE GATT cache refresh failed", throwable)
        }.getOrDefault(false)

        Log.d(TAG, "BLE GATT cache refresh requested result=$refreshed")
    }

    private data class ActiveConnection(
        val gatt: BluetoothGatt,
        val characteristic: BluetoothGattCharacteristic,
    )
}

private class MissingGattSchemaException(message: String) : IOException(message)

private fun describeGattServices(gatt: BluetoothGatt): String {
    return gatt.services.joinToString(separator = "; ") { service ->
        "${service.uuid}[${describeCharacteristics(service)}]"
    }.ifBlank { "none" }
}

private fun describeCharacteristics(service: BluetoothGattService): String {
    return service.characteristics.joinToString(separator = ", ") { characteristic ->
        "${characteristic.uuid}:${characteristicPropertiesText(characteristic.properties)}"
    }.ifBlank { "none" }
}

private fun characteristicPropertiesText(properties: Int): String {
    val names = buildList {
        if (properties and BluetoothGattCharacteristic.PROPERTY_READ != 0) add("read")
        if (properties and BluetoothGattCharacteristic.PROPERTY_WRITE != 0) add("write")
        if (properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0) {
            add("write_no_response")
        }
        if (properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0) add("notify")
        if (properties and BluetoothGattCharacteristic.PROPERTY_INDICATE != 0) add("indicate")
    }

    return names.joinToString("|").ifBlank { "0x${properties.toString(16)}" }
}

fun requiredBluetoothPermissions(): Array<String> {
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        arrayOf(
            Manifest.permission.BLUETOOTH_SCAN,
            Manifest.permission.BLUETOOTH_CONNECT,
        )
    } else {
        arrayOf(Manifest.permission.ACCESS_FINE_LOCATION)
    }
}

private object BluetoothStatusCodes {
    const val SUCCESS = 0
}
