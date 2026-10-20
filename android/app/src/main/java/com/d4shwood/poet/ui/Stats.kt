package com.d4shwood.poet.ui

import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

/** ▤ İstatistikler: totals, then the per-week (or per-year) bars and the hours chart. */
@Composable
fun StatsScreen(ui: Ui) {
    val s = ui.spaceInfo.stats
    Frame(ui, null, {}, listOf(Key(ui.s("k_back")) { ui.go(Screen.Main) })) {
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState())) {
            TermBox(ui.title(Item.Stats), Modifier.fillMaxWidth()) {
                Column {
                    val count = if (ui.lang == "tr") "${s.poems} şiir" else if (s.poems == 1u) "1 poem" else "${s.poems} poems"
                    Txt(line(
                        " ${ui.t("total")}: " to span(T.MUTED),
                        count to span(T.ACCENT),
                        " · ${s.lines} ${ui.t("lines")} · ${s.words} ${ui.t("words")}" to span(T.FG),
                    ))
                    Txt(line(
                        " ${ui.t("streak")}: " to span(T.MUTED),
                        "${s.streak}" to span(T.ACCENT),
                        "  (max ${s.longestStreak})" to span(T.MUTED),
                    ))
                }
            }
            val cell = cellWidth()
            BoxWithConstraints(Modifier.fillMaxWidth()) {
                val cells = (maxWidth / cell).toInt() - 2
                Column {
                    TermBox(ui.t(if (ui.space == null) "per_week" else "per_year"), Modifier.fillMaxWidth(), focused = false) {
                        Column {
                            val max = (s.perGroup.maxOfOrNull { it.count } ?: 1uL).coerceAtLeast(1uL)
                            val barW = (cells - 18).coerceAtLeast(4).toULong()
                            // Newest first, as the terminal draws them.
                            for (b in s.perGroup.reversed()) {
                                val len = ((b.count * barW + max - 1uL) / max).toInt()
                                Txt(line(
                                    " ${b.label.padStart(9)} " to span(T.MUTED),
                                    "█".repeat(len) to span(T.ACCENT),
                                    " ${b.count}" to span(T.FG),
                                ), maxLines = 1)
                            }
                        }
                    }
                    TermBox(ui.t("by_hour"), Modifier.fillMaxWidth(), focused = false) {
                        Column {
                            val height = 8uL
                            val hmax = (s.byHour.maxOrNull() ?: 1uL).coerceAtLeast(1uL)
                            val col = if (cells >= 48) 2 else 1
                            for (level in height downTo 1uL) {
                                val row = s.byHour.joinToString("") { n ->
                                    val filled = n > 0uL && (n * height + hmax - 1uL) / hmax >= level
                                    (if (filled) "█" else " ").padEnd(col)
                                }
                                Txt(" $row", color = T.ACCENT, maxLines = 1)
                            }
                            val axis = (0 until 24).joinToString("") { h ->
                                if (h % 6 == 0) "%02d".format(h).padEnd(col * 6).take(col * 6) else ""
                            }
                            Txt(" $axis", color = T.MUTED, maxLines = 1)
                        }
                    }
                }
            }
        }
    }
}
