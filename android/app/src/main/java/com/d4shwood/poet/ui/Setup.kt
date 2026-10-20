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
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import com.d4shwood.poet.PoetViewModel

private const val NAME = 0
private const val PENS = 1
private const val SERIES = 2
private const val WHERE = 3
private const val REPO = 4
private const val TOKEN = 5
private const val SUMMARY = 6

/** The first-run questions (and ⚙ Ayarlar), drawn like the terminal's setup box. */
@Composable
fun SetupScreen(vm: PoetViewModel, first: Boolean, done: () -> Unit) {
    val start = remember { vm.currentSetup() }
    // Where first: a folder the computer already set up says who you are.
    var step by rememberSaveable { mutableIntStateOf(WHERE) }
    var name by rememberSaveable { mutableStateOf(start.name) }
    var pens by rememberSaveable { mutableStateOf(start.pens.joinToString(", ")) }
    var series by rememberSaveable { mutableStateOf(start.series) }
    var folder by remember { mutableStateOf(start.folder) }
    var github by rememberSaveable { mutableStateOf(start.repo != null || start.folder == null) }
    var repo by rememberSaveable { mutableStateOf(start.repo ?: "") }
    var token by rememberSaveable { mutableStateOf("") }
    val ctx = LocalContext.current
    val hasToken = remember { start.repo != null && com.d4shwood.poet.sync.Secret.load(ctx) != null }
    // The steps this path goes through: a repo and token, or a picked folder.
    val steps = if (github) listOf(WHERE, REPO, TOKEN, NAME, PENS, SERIES, SUMMARY) else listOf(WHERE, NAME, PENS, SERIES, SUMMARY)
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
            github = false
            error = null
            // The folder's .poet/ (written by the computer) fills in the answers.
            runCatching {
                val known = uniffi.poet_core.PoetCore(emptyList(), com.d4shwood.poet.store.SafStore(context, uri).shared())
                val p = known.profile()
                if (p.author.isNotBlank()) {
                    name = p.author
                    series = p.series
                    pens = known.authors().getOrNull(known.mainAuthor().toInt())?.penNames.orEmpty().joinToString(", ")
                }
            }
        }
    }

    fun advance() {
        step = steps[(steps.indexOf(step) + 1).coerceAtMost(steps.size - 1)]
    }

    fun next() {
        error = null
        when (step) {
            NAME -> if (name.isBlank()) error = s("required") else advance()
            WHERE -> if (!github && folder == null) error = s("required") else advance()
            REPO -> if (!Regex("[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+").matches(repo.trim())) error = s("h_repo") else advance()
            TOKEN -> if (token.isBlank() && !hasToken) error = s("required") else advance()
            SUMMARY -> vm.saveSetup(
                PoetViewModel.Setup(
                    name.trim(),
                    pens.split(',').map { it.trim() }.filter { it.isNotEmpty() && it != name.trim() }.distinct(),
                    series,
                    folder.takeIf { !github },
                    repo.trim().takeIf { github },
                    token.takeIf { github && it.isNotBlank() },
                ),
                done,
            )
            else -> advance()
        }
    }

    fun back() {
        error = null
        val i = steps.indexOf(step)
        if (i > 0) step = steps[i - 1] else if (!first) done()
    }

    Column(Modifier.fillMaxSize().padding(8.dp)) {
        val heading = if (first) s("welcome") else s("settings_title")
        TermBox("poetautopublish · $heading", Modifier.fillMaxWidth().weight(1f)) {
            Column(Modifier.padding(horizontal = 6.dp)) {
                Txt(s("step").format(steps.indexOf(step) + 1, steps.size), color = T.FAINT)
                Spacer(Modifier.height(10.dp))
                val (q, help) = when (step) {
                    NAME -> s("q_name") to s("h_name")
                    PENS -> s("q_pens") to s("h_pens")
                    SERIES -> s("q_series") to s("h_series")
                    WHERE -> s("q_where") to s("h_where")
                    REPO -> s("q_repo") to s("h_repo")
                    TOKEN -> s("q_token") to s("h_token")
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
                    WHERE -> Column {
                        ListRow(selected = github, onClick = { github = true; error = null }) { on ->
                            Txt(" ${if (github) "›" else " "} ${s("where_github")}", color = if (on) T.BG else T.FG)
                        }
                        ListRow(selected = !github, onClick = { pick.launch(folder) }) { on ->
                            val chosen = folder?.let { " · " + folderName(it) } ?: ""
                            Txt(" ${if (!github) "›" else " "} ${s("where_folder")}$chosen", color = if (on) T.BG else T.FG)
                        }
                    }
                    REPO -> Field(repo, { repo = it }, ::next)
                    TOKEN -> Column {
                        Field(token, { token = it }, ::next, secret = true)
                        if (hasToken && token.isEmpty()) Txt("   ${s("token_kept")}", color = T.FAINT)
                    }
                    else -> Column {
                        for ((k, v) in listOf(
                            s("name") to name,
                            s("pens") to pens.ifBlank { "—" },
                            s("series") to series,
                            if (github) s("repo") to repo else s("folder") to (folder?.let(::folderName) ?: "—"),
                        )) Txt(line(k.padEnd(14) to span(T.MUTED), v to span(T.FG)))
                    }
                }
                error?.let { Spacer(Modifier.height(8.dp)); Txt(it, color = T.WARN) }
            }
        }
        FooterKeys(buildList {
            add(Key(if (step == SUMMARY) s("save") else s("next")) { next() })
            if (steps.indexOf(step) > 0 || !first) add(Key(s("back")) { back() })
        })
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, onDone: () -> Unit, secret: Boolean = false) {
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
            visualTransformation = if (secret) PasswordVisualTransformation('•') else VisualTransformation.None,
            keyboardOptions = KeyboardOptions(
                imeAction = ImeAction.Next,
                keyboardType = if (secret) KeyboardType.Password else KeyboardType.Text,
                autoCorrectEnabled = !secret,
            ),
            keyboardActions = KeyboardActions(onNext = { onDone() }, onDone = { onDone() }),
        )
    }
}

/** "primary:Şiirler" → "Şiirler". */
private fun folderName(uri: Uri): String = uri.lastPathSegment?.substringAfter(':')?.ifEmpty { "/" } ?: uri.toString()
