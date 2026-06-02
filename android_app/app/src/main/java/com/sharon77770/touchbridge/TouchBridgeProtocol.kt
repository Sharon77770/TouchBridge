package com.sharon77770.touchbridge

private const val DEFAULT_PROFILE_ID = "default"
private const val COMPACT_MIN_ATT_PAYLOAD_BYTES = 20

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

sealed class MousePadEvent {
    data class Move(val dx: Int, val dy: Int) : MousePadEvent()
    data class MouseDelta(
        val dx: Int,
        val dy: Int,
        val dt: Int,
        val seq: Long,
    ) : MousePadEvent()
    data class Scroll(val dy: Int) : MousePadEvent()
    data class Click(val button: TouchBridgeMouseButton) : MousePadEvent()
    data class Button(
        val button: TouchBridgeMouseButton,
        val action: TouchBridgeMouseButtonAction,
    ) : MousePadEvent()
}

sealed class KeyboardRemoteEvent {
    data class TextInput(
        val text: String,
        val seq: Long,
    ) : KeyboardRemoteEvent()

    data class KeyPress(
        val key: TouchBridgeKeyboardKey,
        val seq: Long,
        val modifiers: Set<TouchBridgeKeyboardModifier> = emptySet(),
    ) : KeyboardRemoteEvent()
}

enum class TouchBridgeMouseButton(val wireName: String) {
    Left("left"),
    Right("right"),
}

enum class TouchBridgeMouseButtonAction(val wireName: String) {
    Down("down"),
    Up("up"),
}

enum class TouchBridgeKeyboardKey(val wireName: String) {
    Backspace("backspace"),
    Enter("enter"),
    Escape("escape"),
    Tab("tab"),
    Control("control"),
    Alt("alt"),
    Shift("shift"),
    Win("win"),
    ArrowLeft("arrow_left"),
    ArrowRight("arrow_right"),
    ArrowUp("arrow_up"),
    ArrowDown("arrow_down"),
    Delete("delete"),
    Insert("insert"),
    Home("home"),
    End("end"),
    PageUp("page_up"),
    PageDown("page_down"),
    F1("f1"),
    F2("f2"),
    F3("f3"),
    F4("f4"),
    F5("f5"),
    F6("f6"),
    F7("f7"),
    F8("f8"),
    F9("f9"),
    F10("f10"),
    F11("f11"),
    F12("f12"),
    A("a"),
    B("b"),
    C("c"),
    D("d"),
    E("e"),
    F("f"),
    G("g"),
    H("h"),
    I("i"),
    J("j"),
    K("k"),
    L("l"),
    M("m"),
    N("n"),
    O("o"),
    P("p"),
    Q("q"),
    R("r"),
    S("s"),
    T("t"),
    U("u"),
    V("v"),
    W("w"),
    X("x"),
    Y("y"),
    Z("z"),
}

enum class TouchBridgeKeyboardModifier(val wireName: String) {
    Control("control"),
    Alt("alt"),
    Shift("shift"),
    Win("win"),
}

fun GestureEvent.toProtocolMessage(): String {
    return "G:${gesture.compactCode()}"
}

fun customButtonSyncMessages(buttons: List<CustomButton>): List<String> {
    val normalized = buttons.normalizedCustomButtonPositions()
    return buildList {
        add("Q:B")
        normalized.forEach { button ->
            add(
                "Q:I:${button.position.toCompactString()}:" +
                    "${compactField(button.id)}:${compactField(button.label)}",
            )
        }
        add("Q:E")
    }
}

fun customButtonEventMessage(buttonId: String): String {
    return "Y:${compactField(buttonId)}"
}

fun MousePadEvent.toProtocolMessages(): List<String> {
    return listOf(
        when (this) {
            is MousePadEvent.Move -> "M:${dx.toCompactString()}:${dy.toCompactString()}"
            is MousePadEvent.MouseDelta -> "D:${seq.toCompactString()}:" +
                "${dx.toCompactString()}:${dy.toCompactString()}:${dt.toCompactString()}"
            is MousePadEvent.Scroll -> "S:${dy.toCompactString()}"
            is MousePadEvent.Click -> "C:${button.compactCode()}"
            is MousePadEvent.Button -> "B:${button.compactCode()}:${action.compactCode()}"
        },
    )
}

fun KeyboardRemoteEvent.toProtocolMessages(): List<String> {
    return when (this) {
        is KeyboardRemoteEvent.TextInput -> compactKeyboardTextMessages(seq, text)
        is KeyboardRemoteEvent.KeyPress -> {
            val base = "K:${seq.toCompactString()}:${key.compactCode()}"
            val modifierPart = modifiers.compactCode()
            listOf(if (modifierPart.isEmpty()) base else "$base:$modifierPart")
        }
    }
}

fun handshakeMessage(deviceId: String): String {
    return "H:${compactField(deviceId)}"
}

fun parseProtocolAck(raw: String): ProtocolAck {
    val trimmed = raw.trim()
    return when {
        trimmed == "A1" -> ProtocolAck(ok = true, message = "executed")
        trimmed.startsWith("A1:") -> {
            ProtocolAck(ok = true, message = parseCompactField(trimmed.substringAfter(':')))
        }
        trimmed == "A0" -> ProtocolAck(ok = false, message = "")
        trimmed.startsWith("A0:") -> {
            ProtocolAck(ok = false, message = parseCompactField(trimmed.substringAfter(':')))
        }
        else -> ProtocolAck(ok = false, message = "Unexpected response: $trimmed")
    }
}

private fun compactKeyboardTextMessages(seq: Long, text: String): List<String> {
    if (text.isEmpty()) {
        return emptyList()
    }

    val seqPart = seq.toCompactString()
    val prefix = "T:$seqPart:"
    val maxBytes = ((COMPACT_MIN_ATT_PAYLOAD_BYTES - prefix.length) / 2)
        .coerceAtLeast(4)
    val messages = mutableListOf<String>()
    val current = ArrayList<Byte>(maxBytes)
    var index = 0

    fun flush() {
        if (current.isEmpty()) {
            return
        }

        messages += prefix + current.toByteArray().toHex()
        current.clear()
    }

    while (index < text.length) {
        val codePoint = text.codePointAt(index)
        val chunk = String(Character.toChars(codePoint)).encodeToByteArray()

        if (current.isNotEmpty() && current.size + chunk.size > maxBytes) {
            flush()
        }

        current.addAll(chunk.toList())
        index += Character.charCount(codePoint)
    }

    flush()
    return messages
}

private fun TouchBridgeGesture.compactCode(): String {
    return when (this) {
        TouchBridgeGesture.Tap -> "0"
        TouchBridgeGesture.DoubleTap -> "1"
        TouchBridgeGesture.LongPress -> "2"
        TouchBridgeGesture.SwipeUp -> "3"
        TouchBridgeGesture.SwipeDown -> "4"
        TouchBridgeGesture.SwipeLeft -> "5"
        TouchBridgeGesture.SwipeRight -> "6"
        TouchBridgeGesture.TwoFingerTap -> "7"
        TouchBridgeGesture.TwoFingerSwipeLeft -> "8"
        TouchBridgeGesture.TwoFingerSwipeRight -> "9"
        TouchBridgeGesture.ThreeFingerTap -> "a"
    }
}

private fun TouchBridgeMouseButton.compactCode(): String {
    return when (this) {
        TouchBridgeMouseButton.Left -> "L"
        TouchBridgeMouseButton.Right -> "R"
    }
}

private fun TouchBridgeMouseButtonAction.compactCode(): String {
    return when (this) {
        TouchBridgeMouseButtonAction.Down -> "D"
        TouchBridgeMouseButtonAction.Up -> "U"
    }
}

private fun TouchBridgeKeyboardKey.compactCode(): String {
    return when (this) {
        TouchBridgeKeyboardKey.Backspace -> "B"
        TouchBridgeKeyboardKey.Enter -> "E"
        TouchBridgeKeyboardKey.Escape -> "Esc"
        TouchBridgeKeyboardKey.Tab -> "Tab"
        TouchBridgeKeyboardKey.Control -> "Ctrl"
        TouchBridgeKeyboardKey.Alt -> "Alt"
        TouchBridgeKeyboardKey.Shift -> "Shift"
        TouchBridgeKeyboardKey.Win -> "Win"
        TouchBridgeKeyboardKey.ArrowLeft -> "Left"
        TouchBridgeKeyboardKey.ArrowRight -> "Right"
        TouchBridgeKeyboardKey.ArrowUp -> "Up"
        TouchBridgeKeyboardKey.ArrowDown -> "Down"
        TouchBridgeKeyboardKey.Delete -> "Del"
        TouchBridgeKeyboardKey.Insert -> "Ins"
        TouchBridgeKeyboardKey.Home -> "Home"
        TouchBridgeKeyboardKey.End -> "End"
        TouchBridgeKeyboardKey.PageUp -> "PgUp"
        TouchBridgeKeyboardKey.PageDown -> "PgDn"
        TouchBridgeKeyboardKey.F1 -> "F1"
        TouchBridgeKeyboardKey.F2 -> "F2"
        TouchBridgeKeyboardKey.F3 -> "F3"
        TouchBridgeKeyboardKey.F4 -> "F4"
        TouchBridgeKeyboardKey.F5 -> "F5"
        TouchBridgeKeyboardKey.F6 -> "F6"
        TouchBridgeKeyboardKey.F7 -> "F7"
        TouchBridgeKeyboardKey.F8 -> "F8"
        TouchBridgeKeyboardKey.F9 -> "F9"
        TouchBridgeKeyboardKey.F10 -> "F10"
        TouchBridgeKeyboardKey.F11 -> "F11"
        TouchBridgeKeyboardKey.F12 -> "F12"
        TouchBridgeKeyboardKey.A -> "KeyA"
        TouchBridgeKeyboardKey.B -> "KeyB"
        TouchBridgeKeyboardKey.C -> "KeyC"
        TouchBridgeKeyboardKey.D -> "KeyD"
        TouchBridgeKeyboardKey.E -> "KeyE"
        TouchBridgeKeyboardKey.F -> "KeyF"
        TouchBridgeKeyboardKey.G -> "KeyG"
        TouchBridgeKeyboardKey.H -> "KeyH"
        TouchBridgeKeyboardKey.I -> "KeyI"
        TouchBridgeKeyboardKey.J -> "KeyJ"
        TouchBridgeKeyboardKey.K -> "KeyK"
        TouchBridgeKeyboardKey.L -> "KeyL"
        TouchBridgeKeyboardKey.M -> "KeyM"
        TouchBridgeKeyboardKey.N -> "KeyN"
        TouchBridgeKeyboardKey.O -> "KeyO"
        TouchBridgeKeyboardKey.P -> "KeyP"
        TouchBridgeKeyboardKey.Q -> "KeyQ"
        TouchBridgeKeyboardKey.R -> "KeyR"
        TouchBridgeKeyboardKey.S -> "KeyS"
        TouchBridgeKeyboardKey.T -> "KeyT"
        TouchBridgeKeyboardKey.U -> "KeyU"
        TouchBridgeKeyboardKey.V -> "KeyV"
        TouchBridgeKeyboardKey.W -> "KeyW"
        TouchBridgeKeyboardKey.X -> "KeyX"
        TouchBridgeKeyboardKey.Y -> "KeyY"
        TouchBridgeKeyboardKey.Z -> "KeyZ"
    }
}

private fun TouchBridgeKeyboardModifier.compactCode(): String {
    return when (this) {
        TouchBridgeKeyboardModifier.Control -> "C"
        TouchBridgeKeyboardModifier.Alt -> "A"
        TouchBridgeKeyboardModifier.Shift -> "S"
        TouchBridgeKeyboardModifier.Win -> "W"
    }
}

private fun Set<TouchBridgeKeyboardModifier>.compactCode(): String {
    return listOf(
        TouchBridgeKeyboardModifier.Control,
        TouchBridgeKeyboardModifier.Alt,
        TouchBridgeKeyboardModifier.Shift,
        TouchBridgeKeyboardModifier.Win,
    )
        .filter { contains(it) }
        .joinToString(separator = "") { it.compactCode() }
}

private fun Int.toCompactString(): String = toString(36)

private fun Long.toCompactString(): String = toString(36)

private fun compactField(value: String): String {
    val builder = StringBuilder(value.length)
    value.encodeToByteArray().forEach { rawByte ->
        val byte = rawByte.toInt() and 0xFF
        val char = byte.toChar()
        if (
            char in 'a'..'z' ||
            char in 'A'..'Z' ||
            char in '0'..'9' ||
            char == '-' ||
            char == '_' ||
            char == '.'
        ) {
            builder.append(char)
        } else {
            builder.append('%')
            builder.append(HEX_DIGITS[byte ushr 4])
            builder.append(HEX_DIGITS[byte and 0x0F])
        }
    }
    return builder.toString()
}

private fun parseCompactField(value: String): String {
    val bytes = ArrayList<Byte>(value.length)
    var index = 0

    while (index < value.length) {
        if (value[index] == '%' && index + 2 < value.length) {
            val high = value[index + 1].hexValueOrNull()
            val low = value[index + 2].hexValueOrNull()
            if (high != null && low != null) {
                bytes += ((high shl 4) or low).toByte()
                index += 3
                continue
            }
        }

        bytes += value[index].code.toByte()
        index++
    }

    return bytes.toByteArray().decodeToString()
}

private fun ByteArray.toHex(): String {
    val builder = StringBuilder(size * 2)
    forEach { rawByte ->
        val byte = rawByte.toInt() and 0xFF
        builder.append(HEX_DIGITS[byte ushr 4])
        builder.append(HEX_DIGITS[byte and 0x0F])
    }
    return builder.toString()
}

private fun Char.hexValueOrNull(): Int? {
    return when (this) {
        in '0'..'9' -> code - '0'.code
        in 'a'..'f' -> code - 'a'.code + 10
        in 'A'..'F' -> code - 'A'.code + 10
        else -> null
    }
}

private val HEX_DIGITS = "0123456789ABCDEF".toCharArray()
