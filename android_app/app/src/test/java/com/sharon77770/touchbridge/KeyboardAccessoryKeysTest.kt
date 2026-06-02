package com.sharon77770.touchbridge

import org.junit.Assert.assertEquals
import org.junit.Test

class KeyboardAccessoryKeysTest {
    @Test
    fun basicPresetPlacesBackspaceBetweenShiftAndWin() {
        val basicKeyCodes = KeyboardAccessoryKeys.presets
            .single { it.id == KeyboardAccessoryPresetId.Basic }
            .keys
            .map { it.keyCode }

        val shiftIndex = basicKeyCodes.indexOf(TouchBridgeKeyboardKey.Shift)
        val backspaceIndex = basicKeyCodes.indexOf(TouchBridgeKeyboardKey.Backspace)
        val winIndex = basicKeyCodes.indexOf(TouchBridgeKeyboardKey.Win)

        assertEquals(shiftIndex + 1, backspaceIndex)
        assertEquals(backspaceIndex + 1, winIndex)
    }
}
