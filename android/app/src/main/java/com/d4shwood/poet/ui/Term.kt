package com.d4shwood.poet.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A run of text in the terminal's style. */
@Composable
fun Txt(
    text: String,
    modifier: Modifier = Modifier,
    color: Color = T.FG,
    bold: Boolean = false,
    italic: Boolean = false,
    maxLines: Int = Int.MAX_VALUE,
) {
    BasicText(
        text,
        modifier,
        style = Cell.copy(
            color = color,
            fontWeight = if (bold) FontWeight.Bold else FontWeight.Normal,
            fontStyle = if (italic) FontStyle.Italic else FontStyle.Normal,
        ),
        maxLines = maxLines,
        overflow = TextOverflow.Ellipsis,
    )
}

@Composable
fun Txt(text: AnnotatedString, modifier: Modifier = Modifier, maxLines: Int = Int.MAX_VALUE) {
    BasicText(text, modifier, style = Cell, maxLines = maxLines, overflow = TextOverflow.Ellipsis)
}

fun span(color: Color, bold: Boolean = false, italic: Boolean = false) = SpanStyle(
    color = color,
    fontWeight = if (bold) FontWeight.Bold else null,
    fontStyle = if (italic) FontStyle.Italic else null,
)

/** Builds a line of differently coloured runs, like a ratatui `Line` of `Span`s. */
fun line(vararg parts: Pair<String, SpanStyle>) = buildAnnotatedString {
    for ((text, style) in parts) withStyle(style) { append(text) }
}

/** The width of one terminal cell (one character of the mono font). */
@Composable
fun cellWidth(): Dp {
    val measurer = rememberTextMeasurer()
    val px = remember(measurer) { measurer.measure("0000000000", Cell).size.width / 10f }
    return with(LocalDensity.current) { px.toDp() }
}

/**
 * A ratatui panel: a thin border, accent when focused, with ` title ` set into
 * the top edge.
 */
@Composable
fun TermBox(
    title: String,
    modifier: Modifier = Modifier,
    focused: Boolean = true,
    content: @Composable BoxScope.() -> Unit,
) {
    val edge = if (focused) T.ACCENT else T.FAINT
    Box(modifier.padding(top = 9.dp)) {
        Box(
            Modifier
                .matchParentSize()
                .border(1.dp, edge),
        )
        Box(Modifier.padding(start = 1.dp, end = 1.dp, top = 12.dp, bottom = 4.dp)) { content() }
        if (title.isNotEmpty()) {
            Txt(
                " $title ",
                Modifier
                    .offset(x = 12.dp, y = (-10).dp)
                    .background(T.BG),
                color = if (focused) T.ACCENT else T.MUTED,
            )
        }
    }
}

/** ` poetautopublish — {space}` with a status on the right. */
@Composable
fun Header(space: String, status: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp), verticalAlignment = Alignment.CenterVertically) {
        Txt(
            line(
                " poetautopublish" to span(T.MUTED),
                " — " to span(T.FAINT),
                space to span(T.ACCENT, bold = true),
            ),
            Modifier.weight(1f),
            maxLines = 1,
        )
        Txt("$status ", color = T.MUTED, maxLines = 1)
    }
}

/** A left-menu row: `› ` and a tinted background when selected. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun MenuRow(
    label: String,
    selected: Boolean,
    color: Color = T.FG,
    bold: Boolean = false,
    focused: Boolean = true,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val on = selected || pressed
    Txt(
        "${if (on) "›" else " "} $label",
        Modifier
            .fillMaxWidth()
            .background(if (on) (if (focused) T.SEL_FOCUSED else T.SEL) else Color.Transparent)
            .combinedClickable(source, indication = null, onLongClick = onLongClick, onClick = onClick)
            .padding(vertical = 3.dp),
        color = color,
        bold = bold,
        maxLines = 1,
    )
}

/** A faint group heading in the menu. */
@Composable
fun SectionRow(text: String) {
    Txt(" $text", Modifier.padding(top = 4.dp), color = T.FAINT, maxLines = 1)
}

/**
 * A list row with the terminal's inverted selection bar (dark text on the bone
 * colour) while it is pressed or selected.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun ListRow(
    selected: Boolean = false,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
    content: @Composable (inverted: Boolean) -> Unit,
) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val on = selected || pressed
    Box(
        Modifier
            .fillMaxWidth()
            .background(if (on) T.ACCENT else Color.Transparent)
            .combinedClickable(source, indication = null, onLongClick = onLongClick, onClick = onClick)
            .padding(vertical = 3.dp),
    ) { content(on) }
}

/** A key hint that can be tapped: `n yeni`. */
data class Key(val label: String, val onClick: () -> Unit)

/** The footer's key hints, ` · `-separated, wrapping like the terminal's. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun FooterKeys(keys: List<Key>, modifier: Modifier = Modifier) {
    FlowRow(modifier.fillMaxWidth().padding(horizontal = 6.dp, vertical = 4.dp)) {
        keys.forEachIndexed { i, k ->
            Row {
                if (i > 0) Txt(" · ", color = T.FAINT)
                Txt(k.label, Modifier.clickable(onClick = k.onClick).padding(vertical = 6.dp), color = T.MUTED)
            }
        }
    }
}

/** Pads or cuts text to exactly `n` cells, like `{:<n}` with `…`. */
fun fit(s: String, n: Int): String = when {
    n <= 0 -> ""
    s.length <= n -> s.padEnd(n)
    else -> s.take(n - 1) + "…"
}

@Composable
fun Gap(cells: Int = 1) = Spacer(Modifier.width(cellWidth() * cells))

