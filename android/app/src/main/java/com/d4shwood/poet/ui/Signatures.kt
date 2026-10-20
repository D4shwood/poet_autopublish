package com.d4shwood.poet.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import uniffi.poet_core.AuthorEdit
import uniffi.poet_core.SigRow

/** ◇ İmzalar: authors and pen names, how many poems each carries, and unclaimed signatures. */
@Composable
fun SignaturesScreen(ui: Ui, prompt: Prompt?, closePrompt: () -> Unit) {
    val rows = ui.core.signatureRows()
    var sel by remember { mutableStateOf(0) }
    // Choosing whose pen name an unclaimed signature is.
    var assigning by remember { mutableStateOf<String?>(null) }
    val authors = ui.core.authors()
    val newAuthor = rows.size // the last row, "+ Yeni yazar"

    fun addAuthor() = ui.ask(Prompt.Text(ui.t("author_name"), "") { name ->
        ui.ask(Prompt.Text(ui.t("first_pen"), "") { pen ->
            if (name.isNotBlank() || pen.isNotBlank()) ui.vm.editAuthor(AuthorEdit.AddAuthor(name, pen))
        })
    })

    fun addPen(author: UInt) = ui.ask(Prompt.Text(ui.t("add_pen"), "") { pen ->
        if (pen.isNotBlank()) ui.vm.editAuthor(AuthorEdit.AddPen(author, pen))
    })

    fun enter(i: Int) {
        when (val r = rows.getOrNull(i)) {
            is SigRow.Author -> addPen(r.index)
            is SigRow.Pen -> addPen(r.author)
            is SigRow.Unknown -> assigning = r.name
            null -> addAuthor()
        }
    }

    val keys = buildList {
        add(Key(ui.s("k_back")) { if (assigning != null) assigning = null else ui.go(Screen.Main) })
        when (val r = rows.getOrNull(sel)) {
            is SigRow.Author -> {
                add(Key(ui.s("k_add_pen")) { addPen(r.index) })
                if (!r.isMain) add(Key(ui.s("k_remove")) {
                    ui.ask(Prompt.Confirm(ui.t("confirm_remove_author")) { ui.vm.editAuthor(AuthorEdit.RemoveAuthor(r.index)) })
                })
            }
            is SigRow.Pen -> add(Key(ui.s("k_remove")) { ui.vm.editAuthor(AuthorEdit.RemovePen(r.author, r.index)) })
            is SigRow.Unknown -> add(Key(ui.s("k_assign")) { assigning = r.name })
            null -> add(Key(ui.t("new_author")) { addAuthor() })
        }
    }

    Frame(ui, prompt, closePrompt, keys) {
        val pen = assigning
        if (pen != null) {
            // “name” kimin imzası?
            TermBox(ui.title(Item.Signatures), Modifier.fillMaxSize()) {
                Column {
                    Txt(" “$pen” ${ui.t("assign_to")}", color = T.WARN)
                    Spacer(Modifier.height(8.dp))
                    authors.forEachIndexed { i, a ->
                        ListRow(onClick = {
                            ui.vm.editAuthor(AuthorEdit.AddPen(i.toUInt(), pen))
                            assigning = null
                        }) { on ->
                            val label = a.name.ifBlank { a.penNames.joinToString(" / ") }
                            Txt("  $label  ", color = if (on) T.BG else T.ACCENT, bold = true)
                        }
                    }
                }
            }
            return@Frame
        }
        TermBox(ui.title(Item.Signatures), Modifier.fillMaxSize()) {
            val cell = cellWidth()
            BoxWithConstraints(Modifier.fillMaxSize()) {
                val cells = (maxWidth / cell).toInt()
                val wide = cells >= 64
                val textW = if (wide) 34 else cells - 7
                LazyColumn(Modifier.fillMaxSize()) {
                    var unknownHeader = false
                    rows.forEachIndexed { i, r ->
                        if (r is SigRow.Unknown && !unknownHeader) {
                            unknownHeader = true
                            item(key = "uh") {
                                Column { Spacer(Modifier.height(18.dp)); Txt(" ${ui.t("unknown_signatures")}", color = T.FAINT) }
                            }
                        }
                        item(key = "r$i") {
                            val (text, count, note, color, bold) = when (r) {
                                is SigRow.Author -> Row5(
                                    " " + if (r.hasName) r.label else ui.t("no_real_name"), r.count,
                                    ui.t(if (r.isMain) "main_author" else "tracked"), T.ACCENT, true,
                                )
                                is SigRow.Pen -> Row5("     ${r.name}", r.count, "", T.FG, false)
                                is SigRow.Unknown -> Row5("     ${r.name}", r.count, ui.t("assign_hint"), T.WARN, false)
                            }
                            SigLine(text, count, note, color, bold, i == sel, textW, wide) {
                                if (i == sel) enter(i) else sel = i
                            }
                        }
                    }
                    item(key = "new") {
                        Column {
                            Spacer(Modifier.height(18.dp))
                            SigLine(" ${ui.t("new_author")}", null, "", T.FG, false, sel == newAuthor, textW, wide) {
                                if (sel == newAuthor) addAuthor() else sel = newAuthor
                            }
                        }
                    }
                }
            }
        }
    }
}

private data class Row5(val text: String, val count: UInt?, val note: String, val color: Color, val bold: Boolean)

@Composable
private fun SigLine(
    text: String, count: UInt?, note: String, color: Color, bold: Boolean,
    selected: Boolean, textW: Int, wide: Boolean, onClick: () -> Unit,
) {
    Column(Modifier.fillMaxWidth().clickable(onClick = onClick).padding(vertical = 2.dp)) {
        Row {
            Txt(
                fit(text, textW),
                Modifier.background(if (selected) T.ACCENT else Color.Transparent),
                color = if (selected) T.BG else color, bold = bold, maxLines = 1,
            )
            count?.let { Txt(" ${it.toString().padStart(3)}  ", color = T.MUTED, maxLines = 1) }
            if (wide && note.isNotEmpty()) Txt(note, color = T.FAINT, maxLines = 1)
        }
        if (!wide && note.isNotEmpty()) Txt("     $note", color = T.FAINT, maxLines = 1)
    }
}
