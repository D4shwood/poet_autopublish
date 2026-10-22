package com.d4shwood.poet

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import android.content.Intent
import androidx.core.content.FileProvider
import com.d4shwood.poet.store.AUTHORS
import com.d4shwood.poet.store.FileStore
import com.d4shwood.poet.sync.Paths
import com.d4shwood.poet.sync.Secret
import com.d4shwood.poet.sync.SyncLock
import com.d4shwood.poet.sync.SyncWorker
import com.d4shwood.poet.sync.githubSync
import kotlinx.coroutines.Job
import com.d4shwood.poet.store.PROFILE
import com.d4shwood.poet.store.PoemStore
import com.d4shwood.poet.store.SafStore
import com.d4shwood.poet.store.WEEKS
import com.d4shwood.poet.store.keyOf
import com.d4shwood.poet.store.splitKey
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.poet_core.AuthorEdit
import uniffi.poet_core.PoetCore
import uniffi.poet_core.Profile
import uniffi.poet_core.SharedTexts
import uniffi.poet_core.editAuthors
import uniffi.poet_core.fileNameFor
import uniffi.poet_core.poemContent
import uniffi.poet_core.profileToml
import uniffi.poet_core.renameWeek
import uniffi.poet_core.setMainAuthor
import uniffi.poet_core.tr
import java.time.LocalTime
import java.time.format.DateTimeFormatter

sealed interface Load {
    data object NotSetUp : Load
    data object Reading : Load
    data class Ready(val core: PoetCore) : Load
    data class Failed(val message: String) : Load
}

class PoetViewModel(app: Application) : AndroidViewModel(app) {
    private val prefs = app.getSharedPreferences("poet", Context.MODE_PRIVATE)

    var folder: Uri? = prefs.getString("folder", null)?.let(Uri::parse)
        private set

    /** "owner/name" of the GitHub repo the poems sync with; the phone keeps its own copy. */
    var repo: String? = prefs.getString("repo", null)
        private set

    private val setUp get() = folder != null || repo != null

    private val _load = MutableStateFlow<Load>(if (!setUp) Load.NotSetUp else Load.Reading)
    val load: StateFlow<Load> = _load

    /** A short message in place of the footer for four seconds, as in the terminal. */
    private val _flash = MutableStateFlow<String?>(null)
    val flash: StateFlow<String?> = _flash

    /** The `.poet/` texts as last read; edits start from these. */
    private var texts = SharedTexts(null, null, null)

    private val store: PoemStore?
        get() = when {
            repo != null -> FileStore(Paths.poems(getApplication()))
            else -> folder?.let { SafStore(getApplication(), it) }
        }

    /** The header's right side: the sync state, as the terminal shows the watcher's. */
    sealed interface Sync {
        data object Local : Sync
        data object Running : Sync
        data class Done(val at: String) : Sync
        data object Offline : Sync
    }

    private val _sync = MutableStateFlow<Sync>(Sync.Local)
    val sync: StateFlow<Sync> = _sync

    fun statusText(st: Sync): String = when (st) {
        Sync.Local -> s("local")
        Sync.Running -> "◐ ${s("syncing")}"
        is Sync.Done -> "● ${s("synced")} ${st.at}"
        Sync.Offline -> "○ ${s("offline")}"
    }

    init {
        _sync.value = if (repo != null) Sync.Running else Sync.Local
        if (setUp) refresh()
        if (repo != null) {
            syncNow()
            SyncWorker.schedule(app)
        }
    }

    private var pendingSync: Job? = null

    /** Syncs a few seconds after the last change, so a burst of saves is one commit. */
    private fun syncSoon() {
        if (repo == null) return
        pendingSync?.cancel()
        pendingSync = viewModelScope.launch {
            delay(5000)
            syncNow()
        }
    }

    fun syncNow() {
        val sync = githubSync(getApplication()) ?: return
        _sync.value = Sync.Running
        viewModelScope.launch {
            val result = withContext(Dispatchers.IO) { runCatching { SyncLock.withLock { sync.sync() } } }
            result.onSuccess { out ->
                _sync.value = Sync.Done(now())
                out.conflicts.forEach { say(s("conflict").format(it)) }
                if (out.pulled > 0 || out.conflicts.isNotEmpty() || _load.value !is Load.Ready) refresh()
            }.onFailure {
                _sync.value = Sync.Offline
                say(it.message ?: it.toString())
            }
        }
    }

    /** Opens a built PDF (synced from the computer) in the phone's PDF viewer. */
    fun openPdf(name: String) {
        val app = getApplication<Application>()
        val f = java.io.File(Paths.pdfs(app), name)
        if (!f.isFile) {
            say(t("no_pdf"))
            return
        }
        val uri = FileProvider.getUriForFile(app, "${app.packageName}.files", f)
        val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/pdf")
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        runCatching { app.startActivity(view) }.onFailure { say(it.message ?: it.toString()) }
    }

    fun hasPdf(name: String): Boolean = repo != null && java.io.File(Paths.pdfs(getApplication()), name).isFile

    /** The UI language: the core's once poems are read, else the phone's. */
    val lang: String
        get() = (_load.value as? Load.Ready)?.core?.lang()
            ?: if (java.util.Locale.getDefault().language == "tr") "tr" else "en"

    fun t(key: String): String = (_load.value as? Load.Ready)?.core?.t(key) ?: tr(lang, key)
    fun s(key: String): String = S.get(lang, key)

    fun refresh() {
        val st = store ?: return
        if (_load.value !is Load.Ready) _load.value = Load.Reading
        viewModelScope.launch {
            _load.value = withContext(Dispatchers.IO) {
                runCatching {
                    texts = st.shared()
                    Load.Ready(PoetCore(st.poems(), texts))
                }.getOrElse { Load.Failed(it.message ?: it.toString()) }
            }
        }
    }

    fun say(message: String) {
        _flash.value = message
        viewModelScope.launch {
            delay(4000)
            if (_flash.value == message) _flash.value = null
        }
    }

    // ------------------------------------------------------------ setup

    /** Where the poems live: a folder on the phone, or a GitHub repo (with its token). */
    data class Setup(
        val name: String,
        val pens: List<String>,
        val series: String,
        val folder: Uri?,
        val repo: String?,
        val token: String?,
    )

    fun currentSetup(): Setup {
        val core = (_load.value as? Load.Ready)?.core
        val p = core?.profile()
        val main = core?.authors()?.getOrNull(core.mainAuthor().toInt())
        return Setup(p?.author ?: "", main?.penNames ?: emptyList(), p?.series ?: "Contra Noir", folder, repo, null)
    }

    /**
     * Saves where the poems live, fetches them when they're on GitHub, then
     * writes who the user is into the folder's `.poet/` and reads it.
     */
    fun saveSetup(s: Setup, done: () -> Unit) {
        val app = getApplication<Application>()
        if (s.repo != null) {
            prefs.edit().putString("repo", s.repo.trim()).remove("folder").apply()
            s.token?.takeIf { it.isNotBlank() }?.let { Secret.save(app, it) }
            repo = s.repo.trim()
            folder = null
        } else {
            val uri = s.folder ?: return
            prefs.edit().putString("folder", uri.toString()).remove("repo").apply()
            folder = uri
            repo = null
        }
        val st = store ?: return
        viewModelScope.launch {
            val fetched = withContext(Dispatchers.IO) {
                githubSync(app)?.let { sync -> runCatching { SyncLock.withLock { sync.sync() } } }
            }
            if (fetched?.isFailure == true) {
                say(fetched.exceptionOrNull()?.message ?: "sync")
                _sync.value = Sync.Offline
            }
            withContext(Dispatchers.IO) {
                val old = st.shared()
                val oldProfile = (_load.value as? Load.Ready)?.core?.profile()
                val profile = Profile(
                    author = s.name.trim(),
                    series = s.series.trim().ifEmpty { "Contra Noir" },
                    lang = oldProfile?.lang ?: "auto",
                    nightCutoff = oldProfile?.nightCutoff ?: "05:00",
                    weekFormat = oldProfile?.weekFormat ?: "",
                )
                st.writeShared(PROFILE, profileToml(profile))
                st.writeShared(AUTHORS, setMainAuthor(old.authors, oldProfile?.author ?: s.name, s.name, s.pens))
            }
            refresh()
            if (repo != null) {
                syncNow()
                SyncWorker.schedule(app)
            } else {
                _sync.value = Sync.Local
            }
            done()
        }
    }

    // ------------------------------------------------------------ the .poet/ files

    fun renameWeek(key: String, name: String) = io {
        it.writeShared(WEEKS, renameWeek(texts.weeks, key, name))
        s("renamed").format(name.trim())
    }

    /** Applies an İmzalar edit; says so when the name is already taken. */
    fun editAuthor(edit: AuthorEdit) {
        val core = (_load.value as? Load.Ready)?.core ?: return
        val toml = editAuthors(texts.authors, core.profile().author, edit)
        if (toml == null) {
            say(t("name_taken"))
            return
        }
        io { it.writeShared(AUTHORS, toml); null }
    }

    fun newCollection(name: String) = io {
        val clean = fileNameFor(name, emptyList())
        it.mkdir(clean)
        null
    }

    fun delete(key: String, title: String) = io {
        it.delete(key)
        s("deleted").format(title)
    }

    private fun io(work: (PoemStore) -> String?) {
        val st = store ?: return
        viewModelScope.launch {
            val msg = withContext(Dispatchers.IO) { runCatching { work(st) }.getOrElse { it.message ?: it.toString() } }
            msg?.let(::say)
            refresh()
            syncSoon()
        }
    }

    // ------------------------------------------------------------ poems

    fun readPoem(key: String): String? = store?.read(key)

    /** The writer's meter view (syllables and rhymes beside the lines), remembered. */
    var meterOn: Boolean
        get() = prefs.getBoolean("meter", false)
        set(v) = prefs.edit().putBoolean("meter", v).apply()

    /**
     * Writes a poem. A new poem gets a free file name from its title; a draft's
     * file follows its title; a signed poem keeps its name. Returns the key.
     */
    suspend fun savePoem(key: String?, collection: String?, lines: List<String>, isDraft: Boolean): String =
        withContext(Dispatchers.IO) {
            val st = store ?: error("No poems folder")
            val title = lines.firstOrNull()?.trim().orEmpty()
            val folderOf = key?.let { splitKey(it).first } ?: collection
            val newKey = when {
                key == null -> keyOf(folderOf, fileNameFor(title, st.names(folderOf)))
                isDraft && splitKey(key).second != fileNameFor(title, emptyList()) -> {
                    val current = splitKey(key).second
                    val name = fileNameFor(title, st.names(folderOf).filter { it != current })
                    keyOf(folderOf, name).also { st.rename(key, it) }
                }
                else -> key
            }
            st.write(newKey, poemContent(lines, isDraft))
            newKey
        }.also { syncSoon() }

    fun now(): String = LocalTime.now().format(DateTimeFormatter.ofPattern("HH.mm"))
}
