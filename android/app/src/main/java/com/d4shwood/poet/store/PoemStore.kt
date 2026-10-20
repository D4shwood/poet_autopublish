package com.d4shwood.poet.store

import android.content.Context
import android.net.Uri
import androidx.documentfile.provider.DocumentFile
import uniffi.poet_core.PoemFile
import uniffi.poet_core.SharedTexts
import uniffi.poet_core.isPoemFileName

/**
 * The poems folder. A key is a poem's file name, or "Folder/name" inside a
 * collection, the same keys the core uses. `.poet/` holds the shared settings.
 */
interface PoemStore {
    fun poems(): List<PoemFile>
    fun shared(): SharedTexts
    fun read(key: String): String?
    fun write(key: String, text: String)
    fun rename(key: String, newKey: String)
    fun delete(key: String)
    /** File names already used in a folder (a collection, or null for the top). */
    fun names(collection: String?): List<String>
    fun mkdir(collection: String)
    fun writeShared(name: String, text: String)
}

const val SHARED_DIR = ".poet"
const val PROFILE = "profile.toml"
const val AUTHORS = "authors.toml"
const val WEEKS = "weeks.toml"

fun keyOf(collection: String?, name: String) = if (collection == null) name else "$collection/$name"

fun splitKey(key: String): Pair<String?, String> =
    key.indexOf('/').let { if (it < 0) null to key else key.substring(0, it) to key.substring(it + 1) }

/** A folder picked with the system folder picker (Storage Access Framework). */
class SafStore(private val context: Context, uri: Uri) : PoemStore {
    private val root = DocumentFile.fromTreeUri(context, uri) ?: error("The poems folder can't be opened")

    override fun poems(): List<PoemFile> {
        val out = mutableListOf<PoemFile>()
        for (f in root.listFiles()) {
            val name = f.name ?: continue
            when {
                f.isFile && isPoemFileName(name) -> text(f)?.let { out += PoemFile(name, null, it, f.lastModified()) }
                f.isDirectory && !name.startsWith('.') && !name.startsWith('_') ->
                    for (g in f.listFiles()) {
                        val n = g.name ?: continue
                        if (g.isFile && isPoemFileName(n)) text(g)?.let { out += PoemFile(n, name, it, g.lastModified()) }
                    }
            }
        }
        return out
    }

    override fun shared(): SharedTexts {
        val dir = root.findFile(SHARED_DIR)
        fun read(name: String) = dir?.findFile(name)?.let(::text)
        return SharedTexts(read(PROFILE), read(AUTHORS), read(WEEKS))
    }

    override fun read(key: String): String? = file(key)?.let(::text)

    override fun write(key: String, text: String) {
        val (collection, name) = splitKey(key)
        val dir = folder(collection, create = true)!!
        val f = dir.findFile(name) ?: dir.createFile(RAW, name) ?: error("Couldn't create $name")
        put(f, text)
    }

    override fun rename(key: String, newKey: String) {
        if (key == newKey) return
        file(key)?.renameTo(splitKey(newKey).second)
    }

    override fun delete(key: String) {
        file(key)?.delete()
    }

    override fun names(collection: String?): List<String> =
        folder(collection, create = false)?.listFiles()?.mapNotNull { it.name } ?: emptyList()

    override fun mkdir(collection: String) {
        folder(collection, create = true)
    }

    override fun writeShared(name: String, text: String) {
        val dir = root.findFile(SHARED_DIR) ?: root.createDirectory(SHARED_DIR) ?: error("Couldn't create $SHARED_DIR")
        val f = dir.findFile(name) ?: dir.createFile(RAW, name) ?: error("Couldn't create $name")
        put(f, text)
    }

    private fun folder(collection: String?, create: Boolean): DocumentFile? = when (collection) {
        null -> root
        else -> root.findFile(collection) ?: if (create) root.createDirectory(collection) else null
    }

    private fun file(key: String): DocumentFile? {
        val (collection, name) = splitKey(key)
        return folder(collection, create = false)?.findFile(name)
    }

    private fun text(f: DocumentFile): String? = runCatching {
        context.contentResolver.openInputStream(f.uri)!!.use { it.readBytes().decodeToString(throwOnInvalidSequence = true) }
    }.getOrNull()

    private fun put(f: DocumentFile, text: String) {
        context.contentResolver.openOutputStream(f.uri, "wt")!!.use { it.write(text.toByteArray()) }
    }

    private companion object {
        // Keeps the file name exactly as given: no extension is added.
        const val RAW = "application/octet-stream"
    }
}
