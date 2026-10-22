package com.d4shwood.poet.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextLayoutResult
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import uniffi.poet_core.isSigned
import uniffi.poet_core.lineCount
import uniffi.poet_core.openPoem
import uniffi.poet_core.signPoem
import uniffi.poet_core.verse
import uniffi.poet_core.wordCount
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.width
import androidx.compose.ui.platform.LocalDensity

private const val AUTOSAVE_MS = 30_000L
private const val UNDO_DEPTH = 200

/** The writing screen: the terminal's distraction-free editor, on a touch screen. */
@Composable
fun WriterScreen(ui: Ui, openKey: String?, collection: String?, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    val saving = remember { Mutex() }
    var key by remember { mutableStateOf(openKey) }
    var title by remember { mutableStateOf("") }
    var body by remember { mutableStateOf(TextFieldValue("")) }
    var isDraft by remember { mutableStateOf(true) }
    var dirty by remember { mutableStateOf(false) }
    var savedAt by remember { mutableStateOf<String?>(null) }
    var typewriter by remember { mutableStateOf(false) }
    var meterOn by remember { mutableStateOf(ui.vm.meterOn) }
    var loaded by remember { mutableStateOf(openKey == null) }
    var prompt by remember { mutableStateOf<Prompt?>(null) }
    val started = remember { System.currentTimeMillis() }
    // A faint line from an old poem on the blank page, until the first keystroke.
    var inspiration by remember {
        mutableStateOf(if (openKey == null) ui.core.inspirationLines().randomOrNull() else null)
    }
    val undo = remember { mutableStateListOf<Pair<String, TextFieldValue>>() }
    val bodyFocus = remember { FocusRequester() }
    val titleFocus = remember { FocusRequester() }
    val flash by ui.vm.flash.collectAsState()

    LaunchedEffect(openKey) {
        if (openKey != null) {
            val raw = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) { ui.vm.readPoem(openKey) } ?: ""
            val d = openPoem(raw)
            title = d.lines.firstOrNull().orEmpty()
            body = TextFieldValue(bodyOf(d.lines)).let { it.copy(selection = TextRange(it.text.length)) }
            isDraft = d.isDraft
            loaded = true
            bodyFocus.requestFocus()
        } else {
            titleFocus.requestFocus()
        }
    }

    fun lines(): List<String> = listOf(title) + if (body.text.isEmpty()) emptyList() else listOf("") + body.text.split("\n")

    fun save(then: () -> Unit = {}) {
        if (!loaded || (title.isBlank() && body.text.isBlank())) {
            then(); return
        }
        scope.launch {
            saving.withLock {
                runCatching { ui.vm.savePoem(key, collection, lines(), isDraft) }
                    .onSuccess { key = it; dirty = false; savedAt = ui.vm.now() }
                    .onFailure { ui.vm.say(it.message ?: it.toString()) }
            }
            then()
        }
    }

    fun leave() = save { ui.vm.refresh(); onBack() }

    fun edited(newTitle: String, newBody: TextFieldValue) {
        // A snapshot at word boundaries and on deletions, as the terminal does.
        val typed = newBody.text.length > body.text.length
        val boundary = typed && newBody.text.getOrNull(newBody.selection.start - 1)?.let { it == ' ' || it == '\n' } == true
        if ((boundary || !typed || newTitle != title) && (newBody.text != body.text || newTitle != title)) {
            undo.add(title to body)
            if (undo.size > UNDO_DEPTH) undo.removeAt(0)
        }
        if (newBody.text != body.text || newTitle != title) {
            dirty = true
            inspiration = null
        }
        title = newTitle
        body = newBody
    }

    fun sign() {
        val ls = lines()
        if (isSigned(ls) || body.text.isBlank()) return
        prompt = Prompt.Confirm(ui.t("confirm_sign")) {
            undo.add(title to body)
            val signed = signPoem(ls, ui.core.profile().author)
            body = TextFieldValue(bodyOf(signed)).let { it.copy(selection = TextRange(it.text.length)) }
            isDraft = false
            dirty = true
            save()
        }
    }

    LaunchedEffect(Unit) {
        while (true) {
            delay(AUTOSAVE_MS)
            if (dirty) save()
        }
    }
    BackHandler { if (prompt != null) prompt = null else leave() }

    val ls = lines()
    val signed = isSigned(ls)
    val verseInfo = remember(ls, meterOn) { if (meterOn) verse(ls) else null } ?: uniffi.poet_core.Verse(null, 0u, 0u, emptyList())
    val scroll = rememberScrollState()
    var layout by remember { mutableStateOf<TextLayoutResult?>(null) }
    var viewport by remember { mutableStateOf(0) }
    // Typewriter mode keeps the line being written in the middle of the screen.
    LaunchedEffect(body.selection, typewriter, layout) {
        val l = layout ?: return@LaunchedEffect
        if (!typewriter) return@LaunchedEffect
        val y = l.getCursorRect(body.selection.start.coerceIn(0, body.text.length)).top
        scroll.animateScrollTo((y - viewport / 2).toInt().coerceAtLeast(0))
    }

    Column(Modifier.fillMaxSize()) {
        BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.TopCenter) {
            viewport = constraints.maxHeight
            val cell = cellWidth()
            Column(
                Modifier
                    .widthIn(max = cell * 76)
                    .fillMaxSize()
                    .verticalScroll(scroll)
                    .padding(horizontal = 10.dp),
            ) {
                Spacer(Modifier.height(22.dp))
                BasicTextField(
                    title, { edited(it.replace("\n", ""), body) },
                    Modifier.fillMaxWidth().focusRequester(titleFocus),
                    textStyle = Cell.copy(color = T.ACCENT, fontWeight = FontWeight.Bold),
                    cursorBrush = SolidColor(T.ACCENT),
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Next),
                    keyboardActions = KeyboardActions(onNext = { bodyFocus.requestFocus() }),
                    decorationBox = { inner ->
                        Box {
                            if (title.isEmpty()) Txt(ui.s("w_title"), color = T.FAINT)
                            inner()
                        }
                    },
                )
                Spacer(Modifier.height(18.dp))
                val density = LocalDensity.current
                Row {
                BasicTextField(
                    body, { edited(title, it) },
                    Modifier.weight(1f).focusRequester(bodyFocus),
                    textStyle = Cell.copy(color = T.FG),
                    cursorBrush = SolidColor(T.ACCENT),
                    keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences),
                    onTextLayout = { layout = it },
                    decorationBox = { inner ->
                        Box {
                            inspiration?.takeIf { body.text.isEmpty() }?.let {
                                Txt(it, color = T.FAINT, italic = true)
                            }
                            inner()
                        }
                    },
                )
                // The meter view: each line's syllables and rhyme on its right, as the
                // terminal's Ctrl+E. Lines 0 and 1 are the title and the gap after it.
                val l = layout
                if (meterOn && l != null) {
                    Box(Modifier.width(cell * 7)) {
                        val starts = body.text.split("\n").runningFold(0) { at, line -> at + line.length + 1 }
                        for (m in verseInfo.marks) {
                            val k = m.line.toInt() - 2
                            val offset = starts.getOrNull(k)?.coerceAtMost(body.text.length) ?: continue
                            val top = l.getLineTop(l.getLineForOffset(offset))
                            Txt(
                                (if (m.off) "⚠" else " ") + m.syllables.toString().padStart(2) + " " + (m.rhyme ?: ""),
                                Modifier.offset(y = with(density) { top.toDp() }),
                                color = if (m.off) T.WARN else T.FAINT,
                                maxLines = 1,
                            )
                        }
                    }
                }
                }
                // Room to scroll the last line up to the middle.
                Spacer(Modifier.height(if (typewriter) 400.dp else 120.dp))
            }
        }
        // The status line, as the terminal's: counts, time, save state, draft or signed.
        val mins = (System.currentTimeMillis() - started) / 60_000
        val saved = when {
            savedAt == null -> ui.t("unsaved")
            dirty -> "${ui.t("saved")} $savedAt *"
            else -> "${ui.t("saved")} $savedAt"
        }
        val state = if (signed) ui.t("signed") else ui.t("draft")
        val status = "${lineCount(ls)} ${ui.t("lines")} · ${wordCount(ls)} ${ui.t("words")} · $mins ${ui.t("min")} · " +
            "${ui.vm.now()} · $saved · $state" + (if (typewriter) " · ⌨" else "") +
            if (!meterOn) "" else verseInfo.meter?.let { " · ${ui.t("syllables")} $it ${verseInfo.fits}/${verseInfo.of}" }
                ?: " · ${ui.t("free_verse")}"
        Txt(" $status", Modifier.padding(top = 2.dp, start = 2.dp, end = 2.dp), color = T.FAINT, maxLines = 2)
        val p = prompt
        when {
            p != null -> PromptLine(ui, p) { prompt = null }
            flash != null -> Txt(" $flash", Modifier.padding(vertical = 4.dp), color = T.ACCENT)
            else -> FooterKeys(buildList {
                add(Key(ui.s("w_undo")) {
                    undo.removeLastOrNull()?.let { (t, b) -> title = t; body = b; dirty = true }
                })
                add(Key(ui.s("w_save")) { save() })
                if (!signed) add(Key(ui.s("w_sign")) { sign() })
                add(Key(ui.s("w_typewriter")) { typewriter = !typewriter })
                add(Key(ui.s("w_meter")) { meterOn = !meterOn; ui.vm.meterOn = meterOn })
                add(Key(ui.s("w_back")) { leave() })
            })
        }
    }
}

/** The body lines after the title, without the one blank line that follows it. */
private fun bodyOf(lines: List<String>): String {
    val rest = lines.drop(1)
    return (if (rest.firstOrNull()?.isBlank() == true) rest.drop(1) else rest).joinToString("\n")
}
