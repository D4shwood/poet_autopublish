package com.d4shwood.poet

import android.app.Application
import android.content.Context
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.d4shwood.poet.store.AUTHORS
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

    private val _load = MutableStateFlow<Load>(if (folder == null) Load.NotSetUp else Load.Reading)
    val load: StateFlow<Load> = _load

    /** A short message in place of the footer for four seconds, as in the terminal. */
    private val _flash = MutableStateFlow<String?>(null)
    val flash: StateFlow<String?> = _flash

    /** The `.poet/` texts as last read; edits start from these. */
    private var texts = SharedTexts(null, null, null)

    private val store: PoemStore? get() = folder?.let { SafStore(getApplication(), it) }

    init {
        if (folder != null) refresh()
    }

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

    data class Setup(val name: String, val pens: List<String>, val series: String, val folder: Uri?)

    fun currentSetup(): Setup {
        val core = (_load.value as? Load.Ready)?.core
        val p = core?.profile()
        val main = core?.authors()?.getOrNull(core.mainAuthor().toInt())
        return Setup(p?.author ?: "", main?.penNames ?: emptyList(), p?.series ?: "Contra Noir", folder)
    }

    /** Saves who the user is into the folder's `.poet/`, then reads the folder. */
    fun saveSetup(s: Setup, done: () -> Unit) {
        val uri = s.folder ?: return
        prefs.edit().putString("folder", uri.toString()).apply()
        folder = uri
        val st = SafStore(getApplication(), uri)
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                val old = st.shared()
                val oldProfile = (_load.value as? Load.Ready)?.core?.profile()
                val profile = Profile(
                    author = s.name.trim(),
                    series = s.series.trim().ifEmpty { "Contra Noir" },
                    lang = oldProfile?.lang ?: "auto",
                    nightCutoff = oldProfile?.nightCutoff ?: "05:00",
                )
                st.writeShared(PROFILE, profileToml(profile))
                st.writeShared(AUTHORS, setMainAuthor(old.authors, oldProfile?.author ?: s.name, s.name, s.pens))
            }
            refresh()
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
        }
    }

    // ------------------------------------------------------------ poems

    fun readPoem(key: String): String? = store?.read(key)

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
        }

    fun now(): String = LocalTime.now().format(DateTimeFormatter.ofPattern("HH.mm"))
}
