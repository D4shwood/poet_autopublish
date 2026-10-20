package com.d4shwood.poet.ui

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.d4shwood.poet.PoetViewModel

private const val NAME = 0
private const val PENS = 1
private const val SERIES = 2
private const val FOLDER = 3
private const val SUMMARY = 4
private const val STEPS = 5

/** The first-run questions (and ⚙ Ayarlar), drawn like the terminal's setup box. */
@Composable
fun SetupScreen(vm: PoetViewModel, first: Boolean, done: () -> Unit) {
    val start = remember { vm.currentSetup() }
    var step by rememberSaveable { mutableIntStateOf(NAME) }
    var name by rememberSaveable { mutableStateOf(start.name) }
    var pens by rememberSaveable { mutableStateOf(start.pens.joinToString(", ")) }
    var series by rememberSaveable { mutableStateOf(start.series) }
    var folder by remember { mutableStateOf(start.folder) }
    var error by remember { mutableStateOf<String?>(null) }
    val context = LocalContext.current
    val s = { key: String -> vm.s(key) }

    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri: Uri? ->
        if (uri != null) {
            // Keep access to the folder after the app restarts.
            context.contentResolver.takePersistableUriPermission(
                uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
            folder = uri
            error = null
        }
    }

    fun next() {
        error = null
        when (step) {
            NAME -> if (name.isBlank()) error = s("required") else step = PENS
            FOLDER -> if (folder == null) error = s("required") else step = SUMMARY
            SUMMARY -> vm.saveSetup(
                PoetViewModel.Setup(
                    name.trim(),
                    pens.split(',').map { it.trim() }.filter { it.isNotEmpty() && it != name.trim() }.distinct(),
                    series, folder,
                ),
                done,
            )
            else -> step++
        }
    }

    fun back() {
        error = null
        if (step > NAME) step-- else if (!first) done()
    }

    Column(Modifier.fillMaxSize().padding(8.dp)) {
        val heading = if (first) s("welcome") else s("settings_title")
        TermBox("poetautopublish · $heading", Modifier.fillMaxWidth().weight(1f)) {
            Column(Modifier.padding(horizontal = 6.dp)) {
                Txt(s("step").format(step + 1, STEPS), color = T.FAINT)
                Spacer(Modifier.height(10.dp))
                val (q, help) = when (step) {
                    NAME -> s("q_name") to s("h_name")
                    PENS -> s("q_pens") to s("h_pens")
                    SERIES -> s("q_series") to s("h_series")
                    FOLDER -> s("q_folder") to s("h_folder")
                    else -> s("q_summary") to s("h_summary")
                }
                Txt(q, bold = true)
                Spacer(Modifier.height(6.dp))
                Txt(help, color = T.MUTED)
                Spacer(Modifier.height(14.dp))
                when (step) {
                    NAME -> Field(name, { name = it }, ::next)
                    PENS -> Field(pens, { pens = it }, ::next)
                    SERIES -> Field(series, { series = it }, ::next)
                    FOLDER -> Row {
                        Txt(" › ", color = T.ACCENT)
                        Txt(folder?.let(::folderName) ?: "—", color = if (folder == null) T.FAINT else T.FG)
                    }
                    else -> Column {
                        for ((k, v) in listOf(
                            s("name") to name,
                            s("pens") to pens.ifBlank { "—" },
                            s("series") to series,
                            s("folder") to (folder?.let(::folderName) ?: "—"),
                        )) Txt(line(k.padEnd(14) to span(T.MUTED), v to span(T.FG)))
                    }
                }
                error?.let { Spacer(Modifier.height(8.dp)); Txt(it, color = T.WARN) }
            }
        }
        FooterKeys(buildList {
            if (step == FOLDER) add(Key(s("pick_folder")) { pick.launch(folder) })
            add(Key(if (step == SUMMARY) s("save") else s("next")) { next() })
            if (step > NAME || !first) add(Key(s("back")) { back() })
        })
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, onDone: () -> Unit) {
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    Row {
        Txt(" › ", color = T.ACCENT)
        BasicTextField(
            value, onChange,
            Modifier.fillMaxWidth().focusRequester(focus),
            textStyle = Cell.copy(color = T.FG),
            cursorBrush = SolidColor(T.ACCENT),
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Next),
            keyboardActions = KeyboardActions(onNext = { onDone() }, onDone = { onDone() }),
        )
    }
}

/** "primary:Şiirler" → "Şiirler". */
private fun folderName(uri: Uri): String = uri.lastPathSegment?.substringAfter(':')?.ifEmpty { "/" } ?: uri.toString()
