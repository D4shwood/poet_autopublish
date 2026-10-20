package com.d4shwood.poet

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.poet_core.AuthorEntry
import uniffi.poet_core.PoetLibrary
import uniffi.poet_core.PoemFile
import uniffi.poet_core.Profile
import uniffi.poet_core.isPoemFileName
import uniffi.poet_core.library
import uniffi.poet_core.signatureNow

/** Who the user is and where the poems live, kept in shared preferences. */
data class Settings(
    val folder: Uri?,
    val author: String,
    val penNames: List<String>,
    val series: String,
    val lang: String,
    val nightCutoff: String,
) {
    val complete get() = folder != null && author.isNotBlank()

    fun profile() = Profile(
        author = author,
        authors = listOf(AuthorEntry(name = author, penNames = penNames)),
        series = series,
        lang = lang,
        nightCutoff = nightCutoff,
    )

    companion object {
        private const val PREFS = "poet"

        fun load(context: Context): Settings {
            val p = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            return Settings(
                folder = p.getString("folder", null)?.let(Uri::parse),
                author = p.getString("author", "")!!,
                penNames = p.getString("pen_names", "")!!.split(',').map { it.trim() }.filter { it.isNotEmpty() },
                series = p.getString("series", "Contra Noir")!!,
                lang = p.getString("lang", "auto")!!,
                nightCutoff = p.getString("night_cutoff", "05:00")!!,
            )
        }
    }

    fun save(context: Context) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putString("folder", folder?.toString())
            .putString("author", author.trim())
            .putString("pen_names", penNames.joinToString(", "))
            .putString("series", series.trim())
            .putString("lang", lang)
            .putString("night_cutoff", nightCutoff)
            .apply()
    }
}

/** Reads and writes poem files in the folder the user picked. */
class PoemFolder(private val context: Context, uri: Uri) {
    private val root = DocumentFile.fromTreeUri(context, uri)
        ?: error("The poems folder can't be opened")

    /** Every poem: files at the top level, and one collection per visible subfolder. */
    fun read(): List<PoemFile> {
        val out = mutableListOf<PoemFile>()
        for (f in root.listFiles()) {
            val name = f.name ?: continue
            when {
                f.isFile && isPoemFileName(name) -> text(f)?.let { out += PoemFile(name, null, it) }
                f.isDirectory && !name.startsWith('.') && !name.startsWith('_') ->
                    for (g in f.listFiles()) {
                        val n = g.name ?: continue
                        if (g.isFile && isPoemFileName(n)) text(g)?.let { out += PoemFile(n, name, it) }
                    }
            }
        }
        return out
    }

    /** Writes a new poem named after its title, never replacing an existing file. */
    fun create(title: String, text: String): String {
        val base = fileName(title)
        var name = base
        var n = 2
        while (root.findFile(name) != null) name = "$base ${n++}"
        // application/octet-stream keeps the name as given, with no extension added.
        val file = root.createFile("application/octet-stream", name)
            ?: error("Couldn't create $name")
        context.contentResolver.openOutputStream(file.uri, "wt")!!.use {
            it.write(text.toByteArray())
        }
        return file.name ?: name
    }

    private fun text(f: DocumentFile): String? = runCatching {
        context.contentResolver.openInputStream(f.uri)!!.use { it.readBytes().decodeToString(throwOnInvalidSequence = true) }
    }.getOrNull()

    private fun fileName(title: String): String {
        val s = title.trim().map { if (it == '/' || it == '\\' || it.isISOControl()) '-' else it }
            .joinToString("").trimStart('.', '_').trim()
        return s.ifEmpty { "Adsız" }
    }
}

sealed interface LoadState {
    data object Idle : LoadState
    data object Loading : LoadState
    data class Ready(val library: PoetLibrary) : LoadState
    data class Failed(val message: String) : LoadState
}

class PoetViewModel(app: Application) : AndroidViewModel(app) {
    private val _settings = MutableStateFlow(Settings.load(app))
    val settings: StateFlow<Settings> = _settings

    private val _state = MutableStateFlow<LoadState>(LoadState.Idle)
    val state: StateFlow<LoadState> = _state

    init {
        if (_settings.value.complete) reload()
    }

    fun saveSettings(s: Settings) {
        s.save(getApplication())
        _settings.value = s
        reload()
    }

    fun reload() {
        val s = _settings.value
        val folder = s.folder ?: return
        _state.value = LoadState.Loading
        viewModelScope.launch {
            _state.value = withContext(Dispatchers.IO) {
                runCatching {
                    val files = PoemFolder(getApplication(), folder).read()
                    LoadState.Ready(library(files, s.profile(), emptyMap()))
                }.getOrElse { LoadState.Failed(it.message ?: it.toString()) }
            }
        }
    }

    /** Saves a new poem as a draft, or signed with the current time. */
    fun write(title: String, body: String, sign: Boolean, done: (Result<String>) -> Unit) {
        val s = _settings.value
        val folder = s.folder ?: return
        val text = buildString {
            append(title.trim()).append('\n')
            if (!sign) append("#taslak\n")
            append('\n').append(body.trimEnd()).append('\n')
            if (sign) append('\n').append(signatureNow(s.author)).append('\n')
        }
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching { PoemFolder(getApplication(), folder).create(title, text) }
            }
            done(result)
            reload()
        }
    }
}
