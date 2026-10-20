package com.d4shwood.poet.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/** The same near-black page and warm ink as the PDF volumes. */
private val Noir = darkColorScheme(
    background = Color(0xFF1A1A1A),
    surface = Color(0xFF1A1A1A),
    surfaceVariant = Color(0xFF262626),
    surfaceContainer = Color(0xFF222222),
    onBackground = Color(0xFFE8E2D6),
    onSurface = Color(0xFFE8E2D6),
    onSurfaceVariant = Color(0xFF9C968C),
    primary = Color(0xFFD8CDB8),
    onPrimary = Color(0xFF1A1A1A),
    secondaryContainer = Color(0xFF3A362F),
    onSecondaryContainer = Color(0xFFE8E2D6),
    outline = Color(0xFF55514A),
)

@Composable
fun PoetTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = Noir, content = content)
}
