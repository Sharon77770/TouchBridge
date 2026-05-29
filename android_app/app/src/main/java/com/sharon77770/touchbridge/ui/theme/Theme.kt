package com.sharon77770.touchbridge.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

private val DarkColorScheme = darkColorScheme(
    primary = BridgeTeal80,
    secondary = BridgeAmber80,
    tertiary = BridgeBlue80,
    background = Color(0xFF050A12),
    surface = Color(0xFF0D1420),
    surfaceVariant = Color(0xFF182334),
    onBackground = Color(0xFFEAF2FF),
    onSurface = Color(0xFFEAF2FF),
    onSurfaceVariant = Color(0xFFAAB7C8),
    outline = Color(0xFF3A4A61),
    outlineVariant = Color(0xFF263447),
    error = Color(0xFFFF6B6B),
)

private val LightColorScheme = lightColorScheme(
    primary = BridgeTeal40,
    secondary = BridgeAmber40,
    tertiary = BridgeBlue40,
    background = Color(0xFFF7FAF9),
    surface = Color(0xFFFFFFFF),
    surfaceVariant = Color(0xFFE7EEEC),
    onPrimary = Color.White,
    onSecondary = Color.White,
    onTertiary = Color.White,
    onBackground = Color(0xFF17201F),
    onSurface = Color(0xFF17201F),
    onSurfaceVariant = Color(0xFF455250),
)

@Composable
fun TouchBridgeTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = false,
    content: @Composable () -> Unit
) {
    val colorScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            val context = LocalContext.current
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }

        darkTheme -> DarkColorScheme
        else -> LightColorScheme
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
