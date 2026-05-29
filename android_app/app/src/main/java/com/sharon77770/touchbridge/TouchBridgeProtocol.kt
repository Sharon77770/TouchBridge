package com.sharon77770.touchbridge

import org.json.JSONObject
import org.json.JSONArray

private const val DEFAULT_PROFILE_ID = "default"

data class GestureEvent(
    val deviceId: String,
    val gesture: TouchBridgeGesture,
    val profileId: String = DEFAULT_PROFILE_ID,
    val timestamp: Long = System.currentTimeMillis(),
)

data class ProtocolAck(
    val ok: Boolean,
    val message: String,
)

fun GestureEvent.toProtocolJson(): String {
    return JSONObject()
        .put("type", "gesture_event")
        .put("deviceId", deviceId)
        .put("gesture", gesture.wireName)
        .put("profileId", profileId)
        .put("timestamp", timestamp)
        .toString()
}

fun customButtonSyncJson(
    deviceId: String,
    buttons: List<CustomButton>,
): String {
    val buttonArray = JSONArray()
    buttons.normalizedCustomButtonPositions().forEach { button ->
        buttonArray.put(
            JSONObject()
                .put("id", button.id)
                .put("label", button.label)
                .put("position", button.position),
        )
    }

    return JSONObject()
        .put("type", "custom_button_sync")
        .put("deviceId", deviceId)
        .put("buttons", buttonArray)
        .put("timestamp", System.currentTimeMillis())
        .toString()
}

fun customButtonEventJson(
    deviceId: String,
    buttonId: String,
): String {
    return JSONObject()
        .put("type", "custom_button_event")
        .put("deviceId", deviceId)
        .put("buttonId", buttonId)
        .put("timestamp", System.currentTimeMillis())
        .toString()
}

fun handshakeJson(deviceId: String): String {
    return JSONObject()
        .put("type", "handshake")
        .put("deviceId", deviceId)
        .put("client", "android")
        .put("protocolVersion", 1)
        .put("timestamp", System.currentTimeMillis())
        .toString()
}

fun parseProtocolAck(raw: String): ProtocolAck {
    val json = JSONObject(raw)
    val type = json.optString("type")

    if (type != "ack") {
        return ProtocolAck(
            ok = false,
            message = "Unexpected response type: ${type.ifBlank { "unknown" }}",
        )
    }

    return ProtocolAck(
        ok = json.optBoolean("ok", false),
        message = json.optString("message", ""),
    )
}
