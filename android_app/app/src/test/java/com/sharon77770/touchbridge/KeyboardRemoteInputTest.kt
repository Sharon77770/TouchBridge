package com.sharon77770.touchbridge

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import androidx.compose.ui.text.TextRange

class KeyboardRemoteInputTest {
    @Test
    fun specialKeyPressUsesKeyboardEventFormat() {
        val messages = KeyboardRemoteEvent.KeyPress(
            key = TouchBridgeKeyboardKey.ArrowLeft,
            seq = 1L,
        ).toProtocolMessages()

        assertEquals(listOf("K:1:Left"), messages)
    }

    @Test
    fun shortcutKeyPressIncludesModifiers() {
        val messages = KeyboardRemoteEvent.KeyPress(
            key = TouchBridgeKeyboardKey.C,
            seq = 2L,
            modifiers = setOf(TouchBridgeKeyboardModifier.Control),
        ).toProtocolMessages()

        assertEquals(listOf("K:2:KeyC:C"), messages)
    }

    @Test
    fun shortcutModifierOrderIsStable() {
        val messages = KeyboardRemoteEvent.KeyPress(
            key = TouchBridgeKeyboardKey.ArrowRight,
            seq = 3L,
            modifiers = setOf(
                TouchBridgeKeyboardModifier.Win,
                TouchBridgeKeyboardModifier.Shift,
                TouchBridgeKeyboardModifier.Control,
            ),
        ).toProtocolMessages()

        assertEquals(listOf("K:3:Right:CSW"), messages)
    }

    @Test
    fun typingTextSendsInsertedTextOnly() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("").text,
            nextRaw = keyboardRemoteTextFieldValue("a").text,
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(KeyboardRemoteEvent.TextInput(text = "a", seq = 1L)),
            events,
        )
    }

    @Test
    fun deletingLastVisibleCharacterSendsBackspace() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("a").text,
            nextRaw = keyboardRemoteTextFieldValue("").text,
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(
                KeyboardRemoteEvent.KeyPress(
                    key = TouchBridgeKeyboardKey.Backspace,
                    seq = 1L,
                ),
            ),
            events,
        )
    }

    @Test
    fun deletingEmptyImeFieldSendsBackspace() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("").text,
            nextRaw = "",
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(
                KeyboardRemoteEvent.KeyPress(
                    key = TouchBridgeKeyboardKey.Backspace,
                    seq = 1L,
                ),
            ),
            events,
        )
    }

    @Test
    fun restoringHiddenSentinelDoesNotSendInput() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("").text,
            nextRaw = keyboardRemoteTextFieldValue("").text,
            nextSequence = { ++seq },
        )

        assertTrue(events.isEmpty())
    }

    @Test
    fun composingHangulSendsIntermediateTextImmediately() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("").text,
            nextValue = keyboardRemoteTextFieldValue("ㅅ", composition = TextRange(0, 1)),
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(KeyboardRemoteEvent.TextInput(text = "ㅅ", seq = 1L)),
            events,
        )
    }

    @Test
    fun composingHangulReplacesIntermediateText() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("ㅅ").text,
            nextValue = keyboardRemoteTextFieldValue("사", composition = TextRange(0, 1)),
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(
                KeyboardRemoteEvent.KeyPress(
                    key = TouchBridgeKeyboardKey.Backspace,
                    seq = 1L,
                ),
                KeyboardRemoteEvent.TextInput(text = "사", seq = 2L),
            ),
            events,
        )
    }

    @Test
    fun composingHangulKeepsCommittedPrefixAndReplacesActiveSyllable() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("사고").text,
            nextValue = keyboardRemoteTextFieldValue("사과", composition = TextRange(1, 2)),
            nextSequence = { ++seq },
        )

        assertEquals(
            listOf(
                KeyboardRemoteEvent.KeyPress(
                    key = TouchBridgeKeyboardKey.Backspace,
                    seq = 1L,
                ),
                KeyboardRemoteEvent.TextInput(text = "과", seq = 2L),
            ),
            events,
        )
    }

    @Test
    fun finishingAlreadySentHangulCompositionDoesNotDuplicateText() {
        var seq = 0L

        val events = keyboardRemoteEventsForImeChange(
            previousRaw = keyboardRemoteTextFieldValue("사과").text,
            nextValue = keyboardRemoteTextFieldValue("사과"),
            nextSequence = { ++seq },
        )

        assertEquals(
            emptyList<KeyboardRemoteEvent>(),
            events,
        )
    }
}
