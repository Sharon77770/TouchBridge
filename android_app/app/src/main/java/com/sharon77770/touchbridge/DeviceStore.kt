package com.sharon77770.touchbridge

import android.content.Context
import java.util.Locale
import org.json.JSONArray
import org.json.JSONObject

private const val PREF_SAVED_BLE_DEVICES = "saved_ble_devices"

fun initialDevicesWithSavedBle(
    context: Context,
    usbAttachmentState: UsbAttachmentState = UsbAttachmentState.CableDisconnected,
): List<Device> {
    return sortTouchBridgeDevices(
        initialTouchBridgeDevices(usbAttachmentState) + loadSavedBleDevices(context),
    )
}

fun loadSavedBleDevices(context: Context): List<Device> {
    val raw = context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .getString(PREF_SAVED_BLE_DEVICES, null)
        ?: return emptyList()

    return runCatching {
        decodeSavedBleDevices(raw)
    }.getOrDefault(emptyList())
}

fun saveSavedBleDevices(context: Context, devices: List<Device>) {
    context
        .getSharedPreferences(TOUCHBRIDGE_PREFS, Context.MODE_PRIVATE)
        .edit()
        .putString(PREF_SAVED_BLE_DEVICES, encodeSavedBleDevices(devices))
        .apply()
}

fun mergeScannedBleDevices(
    currentDevices: List<Device>,
    scannedDevices: List<Device>,
): List<Device> {
    val scannedById = scannedDevices
        .filter { it.transport == TransportType.Ble }
        .associateBy { it.id }
    val currentIds = currentDevices.mapTo(mutableSetOf()) { it.id }

    val updatedCurrent = currentDevices.map { current ->
        val scanned = scannedById[current.id] ?: return@map current
        if (current.transport != TransportType.Ble) {
            return@map current
        }

        scanned.copy(
            name = if (current.paired || current.trusted) current.name else scanned.name,
            paired = current.paired || scanned.paired,
            trusted = current.trusted || scanned.trusted,
            available = true,
            lastConnectedAt = current.lastConnectedAt ?: scanned.lastConnectedAt,
            connectionStatus = current.connectionStatus,
            autoConnect = current.autoConnect,
        )
    }

    val newScannedDevices = scannedDevices.filter { scanned -> scanned.id !in currentIds }
    return sortTouchBridgeDevices(updatedCurrent + newScannedDevices)
}

internal fun encodeSavedBleDevices(devices: List<Device>): String {
    val array = JSONArray()
    devices
        .filter { device ->
            device.transport == TransportType.Ble &&
                (device.paired || device.trusted) &&
                bleAddressFromDeviceId(device.id) != null
        }
        .distinctBy { it.id }
        .forEach { device ->
            val item = JSONObject()
                .put("id", device.id)
                .put("name", device.name)
                .put("paired", device.paired)
                .put("trusted", device.trusted)
                .put("autoConnect", device.autoConnect)

            device.lastConnectedAt?.let { lastConnectedAt ->
                item.put("lastConnectedAt", lastConnectedAt)
            }

            array.put(item)
        }

    return array.toString()
}

internal fun decodeSavedBleDevices(raw: String): List<Device> {
    val array = JSONArray(raw)
    val devicesById = linkedMapOf<String, Device>()

    for (index in 0 until array.length()) {
        val item = array.getJSONObject(index)
        val id = item.optString("id").trim()
        val address = bleAddressFromDeviceId(id) ?: continue
        val paired = item.optBoolean("paired", true)
        val trusted = item.optBoolean("trusted", paired)

        if (!paired && !trusted) {
            continue
        }

        val name = item.optString("name").trim().ifBlank {
            "TouchBridge BLE Agent"
        }

        devicesById[bleDeviceId(address)] = bleDeviceFromScan(
            id = bleDeviceId(address),
            name = name,
            trusted = trusted,
            lastConnectedAt = item.optLongOrNull("lastConnectedAt"),
        ).copy(
            paired = paired,
            trusted = trusted,
            available = false,
            autoConnect = item.optBoolean("autoConnect", false),
        )
    }

    return sortTouchBridgeDevices(devicesById.values.toList())
}

private fun sortTouchBridgeDevices(devices: List<Device>): List<Device> {
    return devices.sortedWith(
        compareBy<Device> { it.transport != TransportType.Usb }
            .thenBy { it.name.lowercase(Locale.ROOT) },
    )
}

private fun JSONObject.optLongOrNull(name: String): Long? {
    if (!has(name) || isNull(name)) {
        return null
    }

    return optLong(name).takeIf { it > 0L }
}
