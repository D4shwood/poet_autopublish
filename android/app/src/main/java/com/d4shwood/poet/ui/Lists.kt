package com.d4shwood.poet.ui

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.unit.dp
import uniffi.poet_core.PoemInfo
import uniffi.poet_core.PoemStatus

/** A row whose first cell gets the inverted bar while pressed, as the terminal's selection. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun PressRow(onClick: () -> Unit, onLongClick: () -> Unit, content: @Composable (Boolean) -> Unit) {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    Column(
        Modifier
            .fillMaxWidth()
            .combinedClickable(source, indication = null, onLongClick = onLongClick, onClick = onClick)
            .padding(vertical = 2.dp),
    ) { content(pressed) }
}

fun tag(ui: Ui, p: PoemInfo): String = when (val s = p.status) {
    PoemStatus.Draft -> "${ui.t("draft")} · ${p.lines} ${ui.t("lines")}"
    is PoemStatus.Foreign -> "${ui.t("foreign")}: ${s.author}"
    else -> "${p.lines} ${ui.t("lines")}"
}

/** Opens a poem in the writer; a long press asks to delete it. */
fun poemActions(ui: Ui, p: PoemInfo) = Pair(
    { ui.go(Screen.Writer(p.key, p.collection)) },
    { ui.ask(Prompt.Confirm(ui.s("confirm_delete")) { ui.vm.delete(p.key, p.title) }) },
)

@Composable
fun PoemListPanel(ui: Ui, title: String, poems: List<PoemInfo>, focused: Boolean, empty: String) {
    TermBox(title, Modifier.fillMaxSize(), focused) {
        if (poems.isEmpty()) {
            Txt(" $empty", color = T.MUTED)
            return@TermBox
        }
        val cell = cellWidth()
        BoxWithConstraints(Modifier.fillMaxSize()) {
            val cells = (maxWidth / cell).toInt()
            // The terminal's columns when they fit; on a phone the tag drops below.
            val wide = cells >= 64
            val titleW = if (wide) 34 else cells - 16
            LazyColumn(Modifier.fillMaxSize()) {
                items(poems, key = { it.key }) { p ->
                    val (open, long) = poemActions(ui, p)
                    PressRow(open, long) { on ->
                        Row {
                            Txt(
                                " " + fit(p.title, titleW),
                                Modifier.background(if (on) T.ACCENT else Color.Transparent),
                                color = if (on) T.BG else T.FG, maxLines = 1,
                            )
                            Txt(" " + fit(p.`when`, 14), color = T.MUTED, maxLines = 1)
                            if (wide) Txt(tag(ui, p), color = T.FAINT, maxLines = 1)
                        }
                        if (!wide) Txt("   " + tag(ui, p), color = T.FAINT, maxLines = 1)
                    }
                }
            }
        }
    }
}

/** journal → issue / edition → poem, as `draw_publications` in the terminal. */
@Composable
fun PublicationsPanel(ui: Ui, focused: Boolean) {
    val entries = ui.core.publications(ui.space)
    TermBox(ui.title(Item.Publications), Modifier.fillMaxSize(), focused) {
        if (entries.isEmpty()) {
            Txt(" ${ui.t("no_publications")}", color = T.MUTED)
            return@TermBox
        }
        val cell = cellWidth()
        BoxWithConstraints(Modifier.fillMaxSize()) {
            val cells = (maxWidth / cell).toInt()
            val titleW = if (cells >= 64) 30 else (cells - 22).coerceAtLeast(10)
            LazyColumn(Modifier.fillMaxSize()) {
                var lastJournal: String? = null
                var lastIssue: String? = "\u0000"
                entries.forEachIndexed { i, e ->
                    if (e.journal != lastJournal) {
                        if (lastJournal != null) item(key = "gap$i") { Spacer(Modifier.height(18.dp)) }
                        item(key = "j$i") { Txt(" ${e.journal}", color = T.ACCENT, bold = true) }
                        lastJournal = e.journal
                        lastIssue = "\u0000"
                    }
                    if (e.issue != lastIssue) {
                        e.issue?.let { label -> item(key = "i$i") { Txt("    $label", color = T.MUTED) } }
                        lastIssue = e.issue
                    }
                    val indent = if (e.issue != null) "       " else "    "
                    val p = ui.poems[e.key] ?: return@forEachIndexed
                    item(key = e.key) {
                        val (open, long) = poemActions(ui, p)
                        PressRow(open, long) { on ->
                            Row {
                                Txt(indent, maxLines = 1)
                                Txt(
                                    fit(p.title, titleW),
                                    Modifier.background(if (on) T.ACCENT else Color.Transparent),
                                    color = if (on) T.BG else T.FG, maxLines = 1,
                                )
                                Txt(" " + fit(e.`when`, 12), color = T.FAINT, maxLines = 1)
                                e.shownAuthor?.let { Txt(" $it", color = T.MUTED, maxLines = 1) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/** `/`: search titles and lines in the current space. */
@Composable
fun SearchScreen(ui: Ui) {
    var query by remember { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    val hits = remember(query) { if (query.isBlank()) emptyList() else ui.core.search(query, ui.space) }
    Frame(ui, null, {}, listOf(Key(ui.s("k_back")) { ui.go(Screen.Main) })) {
        TermBox(ui.t("search"), Modifier.fillMaxSize()) {
            Column {
                Row {
                    Txt(" / ", color = T.ACCENT)
                    BasicTextField(
                        query, { query = it },
                        Modifier.fillMaxWidth().focusRequester(focus),
                        textStyle = Cell.copy(color = T.FG),
                        cursorBrush = SolidColor(T.ACCENT),
                        singleLine = true,
                    )
                }
                Spacer(Modifier.height(6.dp))
                if (query.isNotBlank() && hits.isEmpty()) Txt(" ${ui.s("no_hits")}", color = T.MUTED)
                LazyColumn {
                    items(hits) { h ->
                        val p = ui.poems[h.key]
                        PressRow({ if (p != null) ui.go(Screen.Writer(p.key, p.collection)) }, {}) { on ->
                            Row {
                                Txt(
                                    " " + fit(h.title, 22),
                                    Modifier.background(if (on) T.ACCENT else Color.Transparent),
                                    color = if (on) T.BG else T.ACCENT, maxLines = 1,
                                )
                                Txt(" " + h.line, color = T.FG, maxLines = 1)
                            }
                        }
                    }
                }
            }
        }
    }
}
