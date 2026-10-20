package com.d4shwood.poet.sync

import android.util.Base64
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

/** What one sync did. */
data class Outcome(val pulled: Int, val pushed: Int, val conflicts: List<String>, val pdfs: Int)

class SyncError(message: String, val code: Int = 0) : IOException(message)

/**
 * Keeps `poems` (the phone's copy) in step with the `main` branch of a GitHub
 * repo, and `pdfs` with its `pdf` branch, through the REST API — no git on
 * the phone. Local changes go up as one commit per sync.
 */
class GitHubSync(
    private val repo: String,
    private val token: String,
    private val poems: File,
    private val pdfs: File,
    private val stateFile: File,
) {
    private val api = "https://api.github.com/repos/$repo"

    fun sync(): Outcome {
        for (attempt in 1..3) {
            try {
                return once()
            } catch (e: SyncError) {
                // The branch moved while we were committing: start over from the new head.
                if (e.code != 422 || attempt == 3) throw e
            }
        }
        error("unreachable")
    }

    private fun once(): Outcome {
        val state = loadState()
        val head = call("GET", "/git/ref/heads/main").getJSONObject("object").getString("sha")
        val treeSha = call("GET", "/git/commits/$head").getJSONObject("tree").getString("sha")
        val remote = tree(treeSha, recursive = true)
        val local = localFiles()
        val steps = plan(local, remote, state.files)

        val uploads = mutableMapOf<String, String?>() // path → new blob sha, or null to delete
        val conflicts = mutableListOf<String>()
        var pulled = 0
        for (s in steps) {
            when (s) {
                is Step.Download -> { write(s.path, blob(s.sha)); pulled++ }
                is Step.DeleteLocal -> { File(poems, s.path).delete(); pulled++ }
                is Step.Upload -> uploads[s.path] = createBlob(File(poems, s.path).readBytes())
                is Step.DeleteRemote -> uploads[s.path] = null
                is Step.Conflict -> {
                    val mine = File(poems, s.path).readBytes()
                    val copy = conflictName(s.path) { it in remote || File(poems, it).exists() }
                    s.remoteSha?.let { write(s.path, blob(it)) } ?: File(poems, s.path).delete()
                    write(copy, mine)
                    uploads[copy] = createBlob(mine)
                    conflicts += s.path
                    pulled++
                }
            }
        }

        var newRemote = remote
        if (uploads.isNotEmpty()) {
            val entries = JSONArray()
            for ((path, sha) in uploads) {
                entries.put(JSONObject().put("path", path).put("mode", "100644").put("type", "blob").put("sha", sha ?: JSONObject.NULL))
            }
            val newTree = call("POST", "/git/trees", JSONObject().put("base_tree", treeSha).put("tree", entries)).getString("sha")
            val commit = call(
                "POST", "/git/commits",
                JSONObject().put("message", message(uploads.keys)).put("tree", newTree).put("parents", JSONArray().put(head)),
            ).getString("sha")
            // Not forced: if someone pushed meanwhile this fails (422) and the sync starts over.
            call("PATCH", "/git/refs/heads/main", JSONObject().put("sha", commit).put("force", false))
            newRemote = remote.toMutableMap().apply {
                for ((p, sha) in uploads) if (sha == null) remove(p) else put(p, sha)
            }
        }
        val pdfCount = syncPdfs(state)
        saveState(State(newRemote, state.pdfs))
        return Outcome(pulled, uploads.size, conflicts, pdfCount)
    }

    /** Downloads the PDFs that changed on the `pdf` branch; removes ones that are gone. */
    private fun syncPdfs(state: State): Int {
        val ref = try {
            call("GET", "/git/ref/heads/pdf")
        } catch (e: SyncError) {
            if (e.code == 404) return 0 else throw e
        }
        val commit = ref.getJSONObject("object").getString("sha")
        val remote = tree(call("GET", "/git/commits/$commit").getJSONObject("tree").getString("sha"), recursive = false)
            .filterKeys { it.endsWith(".pdf") }
        pdfs.mkdirs()
        var n = 0
        for ((name, sha) in remote) {
            if (state.pdfs[name] != sha || !File(pdfs, name).exists()) {
                File(pdfs, name).writeBytes(blob(sha))
                n++
            }
        }
        pdfs.listFiles()?.filter { it.name !in remote }?.forEach { it.delete() }
        state.pdfs.clear()
        state.pdfs.putAll(remote)
        return n
    }

    // ------------------------------------------------------------ files

    private fun localFiles(): Map<String, String> {
        val out = mutableMapOf<String, String>()
        poems.walkTopDown().filter { it.isFile }.forEach { f ->
            val path = f.relativeTo(poems).invariantSeparatorsPath
            // Our own half-written files are not content.
            if (!f.name.endsWith(".tmp")) out[path] = blobSha(f.readBytes())
        }
        return out
    }

    private fun write(path: String, bytes: ByteArray) {
        val f = File(poems, path)
        f.parentFile?.mkdirs()
        val tmp = File(f.parentFile, ".${f.name}.tmp")
        tmp.writeBytes(bytes)
        if (!tmp.renameTo(f)) {
            f.writeBytes(bytes)
            tmp.delete()
        }
    }

    private fun message(paths: Collection<String>): String {
        val names = paths.map { it.substringAfterLast('/') }
        val shown = names.take(5).joinToString(", ")
        return "$SIDE: $shown" + if (names.size > 5) " +${names.size - 5}" else ""
    }

    // ------------------------------------------------------------ git objects

    private fun tree(sha: String, recursive: Boolean): Map<String, String> {
        val json = call("GET", "/git/trees/$sha" + if (recursive) "?recursive=1" else "")
        val out = mutableMapOf<String, String>()
        val items = json.getJSONArray("tree")
        for (i in 0 until items.length()) {
            val o = items.getJSONObject(i)
            if (o.getString("type") == "blob") out[o.getString("path")] = o.getString("sha")
        }
        return out
    }

    private fun blob(sha: String): ByteArray =
        Base64.decode(call("GET", "/git/blobs/$sha").getString("content"), Base64.DEFAULT)

    private fun createBlob(bytes: ByteArray): String = call(
        "POST", "/git/blobs",
        JSONObject().put("content", Base64.encodeToString(bytes, Base64.NO_WRAP)).put("encoding", "base64"),
    ).getString("sha")

    private fun call(method: String, path: String, body: JSONObject? = null): JSONObject {
        val req = Request.Builder()
            .url(api + path)
            .header("Authorization", "Bearer $token")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .method(method, body?.toString()?.toRequestBody(JSON))
            .build()
        http.newCall(req).execute().use { r ->
            val text = r.body?.string().orEmpty()
            if (!r.isSuccessful) {
                val msg = runCatching { JSONObject(text).getString("message") }.getOrDefault(text.take(200))
                throw SyncError("GitHub ${r.code}: $msg", r.code)
            }
            return JSONObject(text)
        }
    }

    private companion object {
        val JSON = "application/json".toMediaType()
        val http = OkHttpClient.Builder()
            .connectTimeout(15, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .build()
    }

    // ------------------------------------------------------------ state

    /** The remote blobs as of the last sync: the base of the three-way compare. */
    private class State(val files: Map<String, String>, val pdfs: MutableMap<String, String>)

    private fun loadState(): State = runCatching {
        val j = JSONObject(stateFile.readText())
        fun map(o: JSONObject) = o.keys().asSequence().associateWith { o.getString(it) }
        State(map(j.getJSONObject("files")), map(j.getJSONObject("pdfs")).toMutableMap())
    }.getOrElse { State(emptyMap(), mutableMapOf()) }

    private fun saveState(s: State) {
        val j = JSONObject().put("files", JSONObject(s.files)).put("pdfs", JSONObject(s.pdfs as Map<*, *>))
        val tmp = File(stateFile.parentFile, stateFile.name + ".tmp")
        tmp.writeText(j.toString())
        tmp.renameTo(stateFile)
    }
}
