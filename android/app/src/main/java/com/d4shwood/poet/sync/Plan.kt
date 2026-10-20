package com.d4shwood.poet.sync

import java.security.MessageDigest

/** Git's id for a file's content: sha1("blob <size>\0" + bytes). */
fun blobSha(bytes: ByteArray): String {
    val md = MessageDigest.getInstance("SHA-1")
    md.update("blob ${bytes.size}\u0000".toByteArray())
    md.update(bytes)
    return md.digest().joinToString("") { "%02x".format(it) }
}

/** What to do with one path, given its blob on the phone, on the remote, and at the last sync. */
sealed interface Step {
    val path: String
    data class Download(override val path: String, val sha: String) : Step
    data class DeleteLocal(override val path: String) : Step
    data class Upload(override val path: String) : Step
    data class DeleteRemote(override val path: String) : Step
    /** Both sides changed: take the remote's, keep the phone's as a copy that is uploaded. */
    data class Conflict(override val path: String, val remoteSha: String?) : Step
}

/**
 * A three-way comparison, like git's: each side is compared with the state
 * both had at the last sync. Paths missing from a map don't exist there.
 */
fun plan(local: Map<String, String>, remote: Map<String, String>, base: Map<String, String>): List<Step> {
    val steps = mutableListOf<Step>()
    for (path in (local.keys + remote.keys + base.keys).toSortedSet()) {
        val l = local[path]
        val r = remote[path]
        val b = base[path]
        when {
            l == r -> {}
            l == b -> steps += if (r == null) Step.DeleteLocal(path) else Step.Download(path, r)
            r == b -> steps += if (l == null) Step.DeleteRemote(path) else Step.Upload(path)
            // Deleted here, changed there: the change wins.
            l == null -> steps += Step.Download(path, r!!)
            // Changed here, deleted there: the change wins.
            r == null -> steps += Step.Upload(path)
            else -> steps += Step.Conflict(path, r)
        }
    }
    return steps
}

/** "Liman" → "Liman (telefon)", or "(telefon 2)"… when taken. */
fun conflictName(path: String, taken: (String) -> Boolean): String {
    var n = 1
    while (true) {
        val candidate = if (n == 1) "$path ($SIDE)" else "$path ($SIDE $n)"
        if (!taken(candidate)) return candidate
        n++
    }
}

/** This side's name, in commit messages and on conflict copies. */
const val SIDE = "telefon"
