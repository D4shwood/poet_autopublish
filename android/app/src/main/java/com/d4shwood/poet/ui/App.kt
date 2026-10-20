package com.d4shwood.poet.ui

import android.app.Activity
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
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
import androidx.lifecycle.viewmodel.compose.viewModel
import com.d4shwood.poet.Load
import com.d4shwood.poet.PoetViewModel
import uniffi.poet_core.PoemInfo
import uniffi.poet_core.PoemStatus
import uniffi.poet_core.PoetCore

/** The left menu's entries, as in the terminal (src/tui/app.rs `Item`). */
sealed interface Item {
    data object NewPoem : Item
    data object ContinueDraft : Item
    data class Week(val index: Int) : Item
    data object All : Item
    data object Publications : Item
    data class Collection(val name: String) : Item
    data object Book : Item
    data object Series : Item
    data object NewCollection : Item
    data object Unsigned : Item
    data object Drafts : Item
    data object Stats : Item
    data object Signatures : Item
    data object Settings : Item
    data object Quit : Item
}

/** What the bottom line shows instead of key hints: a text prompt or a yes/no question. */
sealed interface Prompt {
    data class Text(val label: String, val initial: String, val onDone: (String) -> Unit) : Prompt
    data class Confirm(val question: String, val onYes: () -> Unit) : Prompt
}

sealed interface Screen {
    data object Main : Screen
    data class Panel(val item: Item) : Screen
    data class Writer(val key: String?, val collection: String?) : Screen
    data object Search : Screen
    data object Setup : Screen
}

/** Everything a screen needs from the app: the core, strings, navigation and prompts. */
class Ui(
    val vm: PoetViewModel,
    val core: PoetCore,
    val space: String?,
    val go: (Screen) -> Unit,
    val ask: (Prompt) -> Unit,
    val enterSpace: (String?) -> Unit,
    val quit: () -> Unit,
) {
    val lang get() = vm.lang
    fun t(key: String) = vm.t(key)
    fun s(key: String) = vm.s(key)
    val poems: Map<String, PoemInfo> by lazy { core.poems().associateBy { it.key } }
    val spaceInfo by lazy { core.space(space) }
    val weeks by lazy { core.weeks() }
    val spaceName: String get() = space ?: core.profile().series

    fun spacePoems(): List<PoemInfo> = spaceInfo.poems.mapNotNull { poems[it] }

    fun withStatus(match: (PoemStatus) -> Boolean) = spacePoems().filter { match(it.status) }

    /** The poems a menu entry lists, in the terminal's order. */
    fun listFor(item: Item): List<PoemInfo> = when (item) {
        is Item.Week -> weeks[item.index].poems.mapNotNull { poems[it] }
        Item.All, Item.Book -> spaceInfo.published.mapNotNull { poems[it] }
        is Item.Collection -> core.space(item.name).poems.mapNotNull { poems[it] }
        Item.Series -> core.space(null).published.mapNotNull { poems[it] }
        Item.Unsigned -> withStatus { it == PoemStatus.Unsigned }
        Item.Drafts, Item.ContinueDraft, Item.NewPoem ->
            withStatus { it == PoemStatus.Draft }.sortedByDescending { it.modified }
        else -> emptyList()
    }

    fun count(item: Item): Int = when (item) {
        Item.Publications -> core.publications(space).size
        is Item.Collection -> core.space(item.name).poems.size
        else -> listFor(item).size
    }

    fun items(): List<Item> = buildList {
        add(Item.NewPoem)
        add(Item.ContinueDraft)
        if (space == null) {
            for (i in weeks.indices.reversed()) add(Item.Week(i))
            add(Item.All)
            add(Item.Publications)
            core.collections().forEach { add(Item.Collection(it)) }
        } else {
            add(Item.Book)
            add(Item.Publications)
            add(Item.Series)
            core.collections().filter { it != space }.forEach { add(Item.Collection(it)) }
        }
        add(Item.NewCollection)
        add(Item.Unsigned)
        add(Item.Drafts)
        add(Item.Stats)
        add(Item.Signatures)
        add(Item.Settings)
        add(Item.Quit)
    }

    fun label(item: Item): String = when (item) {
        Item.NewPoem -> t("new_poem")
        Item.ContinueDraft -> t("continue_draft")
        is Item.Week -> weeks[item.index].let { "  ${it.label}  (${it.poems.size})" }
        Item.All, Item.Book -> "  ${t("all_poems")}  (${count(item)})"
        is Item.Collection -> "  ${item.name}  (${count(item)})"
        Item.NewCollection -> "  ${t("new_collection")}"
        Item.Publications -> "  ${t("publications")}  (${count(item)})"
        Item.Series -> "  ◆ ${core.profile().series}  (${count(item)})"
        Item.Unsigned -> "${t("unsigned")}  (${count(item)})"
        Item.Drafts -> "${t("drafts")}  (${count(item)})"
        Item.Stats -> t("stats")
        Item.Signatures -> t("signatures")
        Item.Settings -> t("settings")
        Item.Quit -> t("quit_item")
    }

    /** A panel's title: the label without its count and leading glyph. */
    fun title(item: Item): String = when (item) {
        Item.Publications -> "$spaceName · ${t("publications").removePrefix("❖ ")}"
        Item.Stats -> t("stats").removePrefix("▤ ")
        Item.Signatures -> t("signatures").removePrefix("◇ ")
        else -> label(item).trim().replace(Regex("""\s+\(\d+\)$"""), "")
    }
}

@Composable
fun PoetApp(vm: PoetViewModel = viewModel()) {
    val load by vm.load.collectAsState()
    var screen by remember { mutableStateOf<Screen>(Screen.Main) }
    var space by rememberSaveable { mutableStateOf<String?>(null) }
    var prompt by remember { mutableStateOf<Prompt?>(null) }
    val activity = LocalContext.current as? Activity

    Box(Modifier.fillMaxSize().background(T.BG).safeDrawingPadding().imePadding()) {
        when (val l = load) {
            Load.NotSetUp -> SetupScreen(vm, first = true) { screen = Screen.Main }
            Load.Reading -> Txt(" ${vm.s("reading")}", Modifier.padding(top = 8.dp), color = T.MUTED)
            is Load.Failed -> Column {
                Txt(" " + vm.s("read_failed").format(l.message), color = T.WARN)
                FooterKeys(listOf(Key(vm.s("k_refresh")) { vm.refresh() }, Key(vm.t("settings")) { screen = Screen.Setup }))
            }
            is Load.Ready -> {
                // A collection that disappeared drops back to the series.
                if (space != null && space !in l.core.collections()) space = null
                val ui = Ui(
                    vm, l.core, space,
                    go = { screen = it },
                    ask = { prompt = it },
                    enterSpace = { space = it; screen = Screen.Main },
                    // ■ End Noir. Poems are saved as they are written, so nothing waits.
                    quit = { activity?.finish() },
                )
                BackHandler(enabled = prompt != null || screen != Screen.Main || space != null) {
                    when {
                        prompt != null -> prompt = null
                        screen != Screen.Main -> screen = Screen.Main
                        else -> space = null
                    }
                }
                when (val s = screen) {
                    Screen.Main -> MainScreen(ui, prompt) { prompt = null }
                    is Screen.Panel -> PanelScreen(ui, s.item, prompt) { prompt = null }
                    is Screen.Writer -> WriterScreen(ui, s.key, s.collection) { screen = Screen.Main }
                    Screen.Search -> SearchScreen(ui)
                    Screen.Setup -> SetupScreen(vm, first = false) { screen = Screen.Main }
                }
            }
        }
    }
}

/** Header, the body, then the footer line (flash message, prompt or key hints). */
@Composable
fun Frame(
    ui: Ui,
    prompt: Prompt?,
    closePrompt: () -> Unit,
    keys: List<Key>,
    body: @Composable () -> Unit,
) {
    val flash by ui.vm.flash.collectAsState()
    Column(Modifier.fillMaxSize()) {
        Header(ui.spaceName, ui.s("local"))
        Box(Modifier.weight(1f).padding(horizontal = 2.dp)) { body() }
        when {
            prompt != null -> PromptLine(ui, prompt, closePrompt)
            flash != null -> Txt(" $flash", Modifier.padding(vertical = 4.dp), color = T.ACCENT)
            else -> FooterKeys(keys)
        }
    }
}

@Composable
fun PromptLine(ui: Ui, prompt: Prompt, close: () -> Unit) {
    when (prompt) {
        is Prompt.Confirm -> Column(Modifier.padding(vertical = 4.dp)) {
            Txt(" ${prompt.question}", color = T.ACCENT, bold = true)
            FooterKeys(listOf(Key(ui.s("yes")) { close(); prompt.onYes() }, Key(ui.s("no")) { close() }))
        }
        is Prompt.Text -> {
            var text by remember(prompt) { mutableStateOf(prompt.initial) }
            val focus = remember { FocusRequester() }
            LaunchedEffect(prompt) { focus.requestFocus() }
            val done = { close(); prompt.onDone(text) }
            Column(Modifier.padding(vertical = 4.dp)) {
                Row {
                    Txt(" ${prompt.label}: ", color = T.ACCENT, bold = true)
                    BasicTextField(
                        text, { text = it },
                        Modifier.weight(1f).focusRequester(focus),
                        textStyle = Cell.copy(color = T.FG),
                        cursorBrush = SolidColor(T.ACCENT),
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = { done() }),
                    )
                }
                FooterKeys(listOf(Key(ui.s("ok")) { done() }, Key(ui.s("cancel")) { close() }))
            }
        }
    }
}

@Composable
fun MainScreen(ui: Ui, prompt: Prompt?, closePrompt: () -> Unit) {
    var selected by remember { mutableStateOf<Item?>(null) }
    val keys = buildList {
        add(Key(ui.s("k_new")) { ui.go(Screen.Writer(null, ui.space)) })
        add(Key(ui.s("k_search")) { ui.go(Screen.Search) })
        (selected as? Item.Week)?.let { w ->
            add(Key(ui.s("k_rename")) { renameWeek(ui, w) })
        }
        add(Key(ui.s("k_refresh")) { ui.vm.refresh() })
    }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        // Wide screens get the terminal's two panels; a phone stacks them.
        val wide = maxWidth >= 600.dp
        Frame(ui, prompt, closePrompt, keys) {
            if (wide) {
                val showing = selected ?: Item.All
                Row(Modifier.fillMaxSize()) {
                    Menu(ui, selected, focused = true, Modifier.weight(0.32f).fillMaxHeight()) { selected = it; open(ui, it, wide = true) }
                    Box(Modifier.weight(0.68f).fillMaxHeight()) { PanelBody(ui, showing, focused = false) }
                }
            } else {
                Menu(ui, selected, focused = true, Modifier.fillMaxSize()) { selected = it; open(ui, it, wide = false) }
            }
        }
    }
}

@Composable
fun Menu(ui: Ui, selected: Item?, focused: Boolean, modifier: Modifier, onSelect: (Item) -> Unit) {
    val items = ui.items()
    TermBox(ui.t("poems"), modifier, focused) {
        LazyColumn(Modifier.fillMaxSize()) {
            items.forEachIndexed { i, item ->
                val prev = items.getOrNull(i - 1)
                // The faint headings and blank lines of the terminal's menu.
                if ((item is Item.Week || item == Item.All) && prev == Item.ContinueDraft) {
                    item(key = "h-weeks") { SectionRow(ui.t("weeks")) }
                }
                if ((item is Item.Collection || item == Item.NewCollection || item == Item.Series) &&
                    prev !is Item.Collection && prev != Item.Series
                ) {
                    item(key = "h-spaces") { SectionRow(ui.t(if (ui.space != null) "other_spaces" else "collections")) }
                }
                if (item == Item.Unsigned || item == Item.Quit) item(key = "gap-$i") { Spacer(Modifier.height(18.dp)) }
                item(key = item.toString()) {
                    val color = when {
                        item == Item.NewPoem -> T.ACCENT
                        item == Item.Unsigned && ui.count(item) > 0 -> T.WARN
                        else -> T.FG
                    }
                    MenuRow(
                        ui.label(item), selected = item == selected, color = color,
                        bold = item == Item.NewPoem, focused = focused,
                        onClick = { onSelect(item) },
                        onLongClick = { if (item is Item.Week) renameWeek(ui, item) },
                    )
                }
            }
        }
    }
}

/** What ⏎ on a menu entry does. */
fun open(ui: Ui, item: Item, wide: Boolean) {
    when (item) {
        Item.NewPoem -> ui.go(Screen.Writer(null, ui.space))
        Item.ContinueDraft -> {
            val newest = ui.listFor(Item.Drafts).firstOrNull()
            ui.go(Screen.Writer(newest?.key, ui.space))
        }
        is Item.Collection -> ui.enterSpace(item.name)
        Item.Series -> ui.enterSpace(null)
        Item.NewCollection -> ui.ask(Prompt.Text(ui.t("collection_name"), "") { name ->
            if (name.isNotBlank()) {
                ui.vm.newCollection(name)
                ui.enterSpace(name.trim())
            }
        })
        Item.Settings -> ui.go(Screen.Setup)
        Item.Quit -> ui.quit()
        else -> if (!wide || item == Item.Stats || item == Item.Signatures) ui.go(Screen.Panel(item))
    }
}

fun renameWeek(ui: Ui, item: Item.Week) {
    val w = ui.weeks[item.index]
    ui.ask(Prompt.Text(ui.t("rename"), w.label) { ui.vm.renameWeek(w.key, it) })
}

@Composable
fun PanelScreen(ui: Ui, item: Item, prompt: Prompt?, closePrompt: () -> Unit) {
    when (item) {
        Item.Stats -> StatsScreen(ui)
        Item.Signatures -> SignaturesScreen(ui, prompt, closePrompt)
        else -> Frame(ui, prompt, closePrompt, listOf(
            Key(ui.s("k_back")) { ui.go(Screen.Main) },
            Key(ui.s("k_new")) { ui.go(Screen.Writer(null, (item as? Item.Collection)?.name ?: ui.space)) },
            Key(ui.s("k_search")) { ui.go(Screen.Search) },
        )) { PanelBody(ui, item, focused = true) }
    }
}

/** The right panel for a menu entry. */
@Composable
fun PanelBody(ui: Ui, item: Item, focused: Boolean) {
    when (item) {
        Item.Publications -> PublicationsPanel(ui, focused)
        Item.Stats, Item.Signatures -> {}
        else -> PoemListPanel(ui, ui.title(item), ui.listFor(item), focused, empty = when (item) {
            Item.NewPoem -> "⏎  ${ui.t("new_poem")}"
            Item.NewCollection -> "⏎  ${ui.t("new_collection")}"
            is Item.Collection -> ui.t("new_in_collection")
            else -> ui.t("empty")
        })
    }
}
