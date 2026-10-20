package com.d4shwood.poet.ui

import android.content.Intent
import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.d4shwood.poet.LoadState
import com.d4shwood.poet.PoetViewModel
import com.d4shwood.poet.R
import com.d4shwood.poet.Settings
import kotlinx.coroutines.launch
import uniffi.poet_core.Library
import uniffi.poet_core.PoemInfo
import uniffi.poet_core.PoemStatus

private sealed interface Screen {
    data object Home : Screen
    data object Setup : Screen
    data object Writer : Screen
    data class Reading(val key: String) : Screen
}

@Composable
fun PoetApp(vm: PoetViewModel = viewModel()) {
    val settings by vm.settings.collectAsState()
    val state by vm.state.collectAsState()
    var screen by remember { mutableStateOf<Screen>(if (settings.complete) Screen.Home else Screen.Setup) }
    BackHandler(enabled = screen != Screen.Home && settings.complete) { screen = Screen.Home }

    when (val s = screen) {
        Screen.Setup -> SetupScreen(settings) {
            vm.saveSettings(it)
            screen = Screen.Home
        }
        Screen.Home -> HomeScreen(
            state = state,
            onRefresh = vm::reload,
            onSettings = { screen = Screen.Setup },
            onNew = { screen = Screen.Writer },
            onOpen = { screen = Screen.Reading(it) },
        )
        Screen.Writer -> WriterScreen(onBack = { screen = Screen.Home }, write = vm::write)
        is Screen.Reading -> {
            val poem = (state as? LoadState.Ready)?.library?.poems?.find { it.key == s.key }
            if (poem == null) screen = Screen.Home else PoemScreen(poem) { screen = Screen.Home }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SetupScreen(initial: Settings, onSave: (Settings) -> Unit) {
    val context = LocalContext.current
    var name by rememberSaveable { mutableStateOf(initial.author) }
    var pens by rememberSaveable { mutableStateOf(initial.penNames.joinToString(", ")) }
    var series by rememberSaveable { mutableStateOf(initial.series) }
    var folder by remember { mutableStateOf(initial.folder) }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri: Uri? ->
        if (uri != null) {
            // Keep access to the folder after the app restarts.
            context.contentResolver.takePersistableUriPermission(
                uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
            folder = uri
        }
    }

    Scaffold(topBar = { TopAppBar(title = { Text(stringResource(R.string.setup_title)) }) }) { pad ->
        Column(
            Modifier.padding(pad).padding(horizontal = 20.dp).fillMaxSize().verticalScroll(rememberScrollState()).imePadding(),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            OutlinedTextField(name, { name = it }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.setup_name)) }, singleLine = true)
            OutlinedTextField(pens, { pens = it }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.setup_pen_names)) }, singleLine = true)
            OutlinedTextField(series, { series = it }, Modifier.fillMaxWidth(), label = { Text(stringResource(R.string.setup_series)) }, singleLine = true)
            OutlinedButton(onClick = { pick.launch(folder) }, Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.setup_pick_folder))
            }
            folder?.let {
                Text(
                    stringResource(R.string.setup_folder_chosen, it.lastPathSegment?.substringAfter(':') ?: it.toString()),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Button(
                onClick = {
                    onSave(
                        initial.copy(
                            folder = folder,
                            author = name.trim(),
                            penNames = pens.split(',').map { it.trim() }.filter { it.isNotEmpty() },
                            series = series.trim().ifEmpty { "Contra Noir" },
                        ),
                    )
                },
                enabled = name.isNotBlank() && folder != null,
                modifier = Modifier.fillMaxWidth(),
            ) { Text(stringResource(R.string.setup_save)) }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun HomeScreen(
    state: LoadState,
    onRefresh: () -> Unit,
    onSettings: () -> Unit,
    onNew: () -> Unit,
    onOpen: (String) -> Unit,
) {
    var tab by rememberSaveable { mutableIntStateOf(0) }
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.app_name)) },
                actions = {
                    IconButton(onRefresh) { Icon(Icons.Filled.Refresh, stringResource(R.string.refresh)) }
                    IconButton(onSettings) { Icon(Icons.Filled.Settings, stringResource(R.string.settings)) }
                },
            )
        },
        bottomBar = {
            NavigationBar {
                listOf(R.string.tab_weeks, R.string.tab_poems, R.string.tab_stats).forEachIndexed { i, label ->
                    NavigationBarItem(selected = tab == i, onClick = { tab = i }, icon = {}, label = { Text(stringResource(label)) })
                }
            }
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = onNew,
                icon = { Icon(Icons.Filled.Add, null) },
                text = { Text(stringResource(R.string.new_poem)) },
            )
        },
    ) { pad ->
        Box(Modifier.padding(pad).fillMaxSize()) {
            when (state) {
                LoadState.Idle, LoadState.Loading -> Column(
                    Modifier.align(Alignment.Center),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    CircularProgressIndicator()
                    Spacer(Modifier.height(12.dp))
                    Text(stringResource(R.string.loading))
                }
                is LoadState.Failed -> Text(
                    stringResource(R.string.load_failed, state.message),
                    Modifier.padding(20.dp),
                    color = MaterialTheme.colorScheme.error,
                )
                is LoadState.Ready -> when (tab) {
                    0 -> WeeksTab(state.library, onOpen)
                    1 -> PoemsTab(state.library, onOpen)
                    else -> StatsTab(state.library)
                }
            }
        }
    }
}

@Composable
private fun WeeksTab(lib: Library, onOpen: (String) -> Unit) {
    if (lib.weeks.isEmpty()) {
        Text(stringResource(R.string.no_weeks), Modifier.padding(20.dp))
        return
    }
    val byKey = lib.poems.associateBy { it.key }
    LazyColumn(contentPadding = PaddingValues(bottom = 96.dp)) {
        // Newest week first, as on a phone you mostly look at the latest.
        items(lib.weeks.reversed(), key = { it.key }) { week ->
            Column(Modifier.padding(horizontal = 20.dp, vertical = 14.dp)) {
                Text(week.label, style = MaterialTheme.typography.titleLarge)
                Text(
                    stringResource(R.string.volume, week.number.toInt()) + " · " +
                        stringResource(R.string.poem_count, week.poems.size),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(6.dp))
                week.poems.mapNotNull { byKey[it] }.forEach { p ->
                    Text(
                        p.title,
                        Modifier.fillMaxWidth().clickable { onOpen(p.key) }.padding(vertical = 6.dp),
                        fontFamily = FontFamily.Serif,
                    )
                }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f))
        }
    }
}

@Composable
private fun PoemsTab(lib: Library, onOpen: (String) -> Unit) {
    val poems = lib.poems.sortedWith(compareByDescending<PoemInfo> { it.date ?: "" }.thenBy { it.title })
    LazyColumn(contentPadding = PaddingValues(bottom = 96.dp)) {
        items(poems, key = { it.key }) { p ->
            Row(
                Modifier.fillMaxWidth().clickable { onOpen(p.key) }.padding(horizontal = 20.dp, vertical = 12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(p.title, fontFamily = FontFamily.Serif)
                    val meta = listOfNotNull(p.collection, p.date?.let(::shortDate), p.signedAs?.takeIf { p.status is PoemStatus.Foreign })
                    if (meta.isNotEmpty()) {
                        Text(
                            meta.joinToString(" · "),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                statusLabel(p.status)?.let {
                    Text(it, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        }
    }
}

@Composable
private fun statusLabel(s: PoemStatus): String? = when (s) {
    PoemStatus.Draft -> stringResource(R.string.status_draft)
    PoemStatus.Unsigned -> stringResource(R.string.status_unsigned)
    else -> null
}

@Composable
private fun StatsTab(lib: Library) {
    val s = lib.stats
    Column(Modifier.padding(20.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Stat(R.string.stat_poems, s.poems.toString())
        Stat(R.string.stat_lines, s.lines.toString())
        Stat(R.string.stat_words, s.words.toString())
        Stat(R.string.stat_streak, s.streak.toString())
        Stat(R.string.stat_longest, s.longestStreak.toString())
        Spacer(Modifier.height(8.dp))
        Text(stringResource(R.string.stat_hours), style = MaterialTheme.typography.titleMedium)
        HourChart(s.byHour.map { it.toLong() })
    }
}

@Composable
private fun Stat(label: Int, value: String) {
    Row(Modifier.fillMaxWidth()) {
        Text(stringResource(label), Modifier.weight(1f), color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.titleMedium)
    }
}

/** Poems per hour of the day as 24 thin bars. */
@Composable
private fun HourChart(hours: List<Long>) {
    val max = (hours.maxOrNull() ?: 0L).coerceAtLeast(1L)
    Row(Modifier.fillMaxWidth().height(96.dp), verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.spacedBy(2.dp)) {
        hours.forEach { n ->
            Box(
                Modifier.weight(1f)
                    .fillMaxHeight(fraction = (n.toFloat() / max).coerceAtLeast(0.02f))
                    .background(if (n > 0) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline),
            )
        }
    }
    Row(Modifier.fillMaxWidth()) {
        listOf("00", "06", "12", "18", "23").forEach { h ->
            Text(h, Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PoemScreen(p: PoemInfo, onBack: () -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = { IconButton(onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) } },
            )
        },
    ) { pad ->
        Column(Modifier.padding(pad).padding(horizontal = 28.dp).verticalScroll(rememberScrollState())) {
            Text(p.title, style = MaterialTheme.typography.headlineSmall, fontFamily = FontFamily.Serif)
            Text("* * *", Modifier.padding(vertical = 8.dp), color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(12.dp))
            // Leading spaces are the poem's indentation, so keep them as they are.
            p.body.forEach { line ->
                Text(line.ifEmpty { " " }, fontFamily = FontFamily.Serif, style = MaterialTheme.typography.bodyLarge)
            }
            Spacer(Modifier.height(28.dp))
            val sign = listOfNotNull(
                p.signedAs,
                p.date?.let { (if (p.approx) "~" else "") + shortDate(it) },
                p.time?.replace(':', '.'),
            ).joinToString(" · ")
            if (sign.isNotEmpty()) {
                Text(sign, Modifier.fillMaxWidth(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            p.publication?.let {
                Text(
                    stringResource(R.string.first_published, it),
                    style = MaterialTheme.typography.bodySmall,
                    fontStyle = FontStyle.Italic,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.height(40.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun WriterScreen(onBack: () -> Unit, write: (String, String, Boolean, (Result<String>) -> Unit) -> Unit) {
    var title by rememberSaveable { mutableStateOf("") }
    var body by rememberSaveable { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    val failedMsg = stringResource(R.string.writer_failed, "%s")

    fun save(sign: Boolean) {
        busy = true
        write(title, body, sign) { result ->
            busy = false
            result.onSuccess { onBack() }
                .onFailure { e -> scope.launch { snackbar.showSnackbar(failedMsg.format(e.message ?: e.toString())) } }
        }
    }

    val plain = TextFieldDefaults.colors(
        focusedContainerColor = Color.Transparent,
        unfocusedContainerColor = Color.Transparent,
        focusedIndicatorColor = Color.Transparent,
        unfocusedIndicatorColor = Color.Transparent,
    )
    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = { IconButton(onBack) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) } },
                actions = {
                    OutlinedButton(onClick = { save(false) }, enabled = !busy && title.isNotBlank()) {
                        Text(stringResource(R.string.writer_save_draft))
                    }
                    Spacer(Modifier.padding(4.dp))
                    Button(onClick = { save(true) }, enabled = !busy && title.isNotBlank() && body.isNotBlank()) {
                        Text(stringResource(R.string.writer_sign))
                    }
                    Spacer(Modifier.padding(4.dp))
                },
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
    ) { pad ->
        Column(Modifier.padding(pad).fillMaxSize().imePadding()) {
            TextField(
                title, { title = it.replace("\n", "") }, Modifier.fillMaxWidth(),
                placeholder = { Text(stringResource(R.string.writer_title)) },
                textStyle = MaterialTheme.typography.headlineSmall.copy(fontFamily = FontFamily.Serif),
                colors = plain, singleLine = true,
            )
            TextField(
                body, { body = it }, Modifier.fillMaxWidth().weight(1f),
                placeholder = { Text(stringResource(R.string.writer_body)) },
                textStyle = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Serif),
                colors = plain,
            )
        }
    }
}

/** "2026-10-05" → "05.10.2026", as poets write it in signatures. */
private fun shortDate(iso: String): String = iso.split('-').let { if (it.size == 3) "${it[2]}.${it[1]}.${it[0]}" else iso }
