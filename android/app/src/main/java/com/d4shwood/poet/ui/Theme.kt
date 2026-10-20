package com.d4shwood.poet.ui

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import com.d4shwood.poet.R

/** The terminal's palette, exactly as in src/tui/ui.rs. */
object T {
    val BG = Color(0xFF151515)
    val FG = Color(0xFFE6E6E6)
    val MUTED = Color(0xFF888888)
    val FAINT = Color(0xFF4A4A4A)
    val ACCENT = Color(0xFFD8CFC0)
    val WARN = Color(0xFFC88A5A)

    /** Background of the selected menu row, when its panel has focus / doesn't. */
    val SEL_FOCUSED = Color(0xFF2C2A26)
    val SEL = Color(0xFF222222)
}

/** Noto Sans Mono, the font the terminal draws with. Italic is slanted, as in Konsole. */
val Mono = FontFamily(
    Font(R.font.noto_sans_mono_regular, FontWeight.Normal),
    Font(R.font.noto_sans_mono_bold, FontWeight.Bold),
)

/** One terminal cell row: every text in the app uses this. */
val Cell = TextStyle(fontFamily = Mono, fontSize = 14.sp, lineHeight = 1.35.em, color = T.FG)

@Composable
fun PoetTheme(content: @Composable () -> Unit) {
    val colors = darkColorScheme(
        background = T.BG,
        surface = T.BG,
        surfaceVariant = T.BG,
        surfaceContainer = T.BG,
        onBackground = T.FG,
        onSurface = T.FG,
        onSurfaceVariant = T.MUTED,
        primary = T.ACCENT,
        onPrimary = T.BG,
        outline = T.FAINT,
        error = T.WARN,
    )
    val type = Typography().let { d ->
        Typography(
            bodyLarge = Cell, bodyMedium = Cell, bodySmall = Cell,
            labelLarge = Cell, labelMedium = Cell, labelSmall = Cell,
            titleLarge = Cell, titleMedium = Cell, titleSmall = Cell,
            headlineSmall = d.headlineSmall.merge(Cell),
        )
    }
    MaterialTheme(colorScheme = colors, typography = type, content = content)
}
