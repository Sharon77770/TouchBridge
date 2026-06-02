package com.sharon77770.touchbridge

enum class KeyboardAccessoryKeyType {
    Normal,
    Modifier,
    Shortcut,
}

enum class KeyboardAccessoryPresetId {
    Basic,
    Navigation,
    Function,
    Shortcut,
}

data class KeyboardAccessoryKeyDefinition(
    val displayLabel: String,
    val keyCode: TouchBridgeKeyboardKey,
    val type: KeyboardAccessoryKeyType,
    val modifiers: Set<TouchBridgeKeyboardModifier> = emptySet(),
    val description: String,
)

data class KeyboardAccessoryPreset(
    val id: KeyboardAccessoryPresetId,
    val keys: List<KeyboardAccessoryKeyDefinition>,
)

object KeyboardAccessoryKeys {
    val presets: List<KeyboardAccessoryPreset> = listOf(
        KeyboardAccessoryPreset(
            id = KeyboardAccessoryPresetId.Basic,
            keys = listOf(
                normal("Esc", TouchBridgeKeyboardKey.Escape, "Escape"),
                normal("Tab", TouchBridgeKeyboardKey.Tab, "Tab"),
                modifier("Ctrl", TouchBridgeKeyboardKey.Control, TouchBridgeKeyboardModifier.Control),
                modifier("Alt", TouchBridgeKeyboardKey.Alt, TouchBridgeKeyboardModifier.Alt),
                modifier("Shift", TouchBridgeKeyboardKey.Shift, TouchBridgeKeyboardModifier.Shift),
                modifier("Win", TouchBridgeKeyboardKey.Win, TouchBridgeKeyboardModifier.Win),
                normal("←", TouchBridgeKeyboardKey.ArrowLeft, "ArrowLeft"),
                normal("↑", TouchBridgeKeyboardKey.ArrowUp, "ArrowUp"),
                normal("↓", TouchBridgeKeyboardKey.ArrowDown, "ArrowDown"),
                normal("→", TouchBridgeKeyboardKey.ArrowRight, "ArrowRight"),
                normal("Del", TouchBridgeKeyboardKey.Delete, "Delete"),
            ),
        ),
        KeyboardAccessoryPreset(
            id = KeyboardAccessoryPresetId.Navigation,
            keys = listOf(
                normal("Ins", TouchBridgeKeyboardKey.Insert, "Insert"),
                normal("Home", TouchBridgeKeyboardKey.Home, "Home"),
                normal("End", TouchBridgeKeyboardKey.End, "End"),
                normal("PgUp", TouchBridgeKeyboardKey.PageUp, "PageUp"),
                normal("PgDn", TouchBridgeKeyboardKey.PageDown, "PageDown"),
            ),
        ),
        KeyboardAccessoryPreset(
            id = KeyboardAccessoryPresetId.Function,
            keys = listOf(
                normal("F1", TouchBridgeKeyboardKey.F1, "F1"),
                normal("F2", TouchBridgeKeyboardKey.F2, "F2"),
                normal("F3", TouchBridgeKeyboardKey.F3, "F3"),
                normal("F4", TouchBridgeKeyboardKey.F4, "F4"),
                normal("F5", TouchBridgeKeyboardKey.F5, "F5"),
                normal("F6", TouchBridgeKeyboardKey.F6, "F6"),
                normal("F7", TouchBridgeKeyboardKey.F7, "F7"),
                normal("F8", TouchBridgeKeyboardKey.F8, "F8"),
                normal("F9", TouchBridgeKeyboardKey.F9, "F9"),
                normal("F10", TouchBridgeKeyboardKey.F10, "F10"),
                normal("F11", TouchBridgeKeyboardKey.F11, "F11"),
                normal("F12", TouchBridgeKeyboardKey.F12, "F12"),
            ),
        ),
        KeyboardAccessoryPreset(
            id = KeyboardAccessoryPresetId.Shortcut,
            keys = listOf(
                shortcut("Win+D", TouchBridgeKeyboardKey.D, TouchBridgeKeyboardModifier.Win, "Show desktop"),
                shortcut("Win+E", TouchBridgeKeyboardKey.E, TouchBridgeKeyboardModifier.Win, "Open Explorer"),
                shortcut("Win+R", TouchBridgeKeyboardKey.R, TouchBridgeKeyboardModifier.Win, "Run"),
                shortcut("Alt+Tab", TouchBridgeKeyboardKey.Tab, TouchBridgeKeyboardModifier.Alt, "Switch window"),
                shortcut("Ctrl+C", TouchBridgeKeyboardKey.C, TouchBridgeKeyboardModifier.Control, "Copy"),
                shortcut("Ctrl+V", TouchBridgeKeyboardKey.V, TouchBridgeKeyboardModifier.Control, "Paste"),
                shortcut("Ctrl+A", TouchBridgeKeyboardKey.A, TouchBridgeKeyboardModifier.Control, "Select all"),
                shortcut("Ctrl+L", TouchBridgeKeyboardKey.L, TouchBridgeKeyboardModifier.Control, "Focus location"),
                shortcut(
                    "Shift+←",
                    TouchBridgeKeyboardKey.ArrowLeft,
                    TouchBridgeKeyboardModifier.Shift,
                    "Extend selection left",
                ),
                shortcut(
                    "Shift+→",
                    TouchBridgeKeyboardKey.ArrowRight,
                    TouchBridgeKeyboardModifier.Shift,
                    "Extend selection right",
                ),
                shortcut(
                    "Win+←",
                    TouchBridgeKeyboardKey.ArrowLeft,
                    TouchBridgeKeyboardModifier.Win,
                    "Snap left",
                ),
                shortcut(
                    "Win+→",
                    TouchBridgeKeyboardKey.ArrowRight,
                    TouchBridgeKeyboardModifier.Win,
                    "Snap right",
                ),
            ),
        ),
    )

    val allKeys: List<KeyboardAccessoryKeyDefinition> = presets.flatMap { it.keys }

    private fun normal(
        displayLabel: String,
        keyCode: TouchBridgeKeyboardKey,
        description: String,
    ): KeyboardAccessoryKeyDefinition {
        return KeyboardAccessoryKeyDefinition(
            displayLabel = displayLabel,
            keyCode = keyCode,
            type = KeyboardAccessoryKeyType.Normal,
            description = description,
        )
    }

    private fun modifier(
        displayLabel: String,
        keyCode: TouchBridgeKeyboardKey,
        modifier: TouchBridgeKeyboardModifier,
    ): KeyboardAccessoryKeyDefinition {
        return KeyboardAccessoryKeyDefinition(
            displayLabel = displayLabel,
            keyCode = keyCode,
            type = KeyboardAccessoryKeyType.Modifier,
            modifiers = setOf(modifier),
            description = modifier.wireName,
        )
    }

    private fun shortcut(
        displayLabel: String,
        keyCode: TouchBridgeKeyboardKey,
        modifier: TouchBridgeKeyboardModifier,
        description: String,
    ): KeyboardAccessoryKeyDefinition {
        return KeyboardAccessoryKeyDefinition(
            displayLabel = displayLabel,
            keyCode = keyCode,
            type = KeyboardAccessoryKeyType.Shortcut,
            modifiers = setOf(modifier),
            description = description,
        )
    }
}
