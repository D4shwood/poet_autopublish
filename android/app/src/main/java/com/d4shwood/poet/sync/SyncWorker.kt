package com.d4shwood.poet.sync

import android.content.Context
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.NetworkType
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import java.io.File
import kotlinx.coroutines.sync.withLock
import java.util.concurrent.TimeUnit

/** Where the synced copy lives in the app's private storage. */
object Paths {
    fun poems(c: Context) = File(c.filesDir, "poems")
    fun pdfs(c: Context) = File(c.filesDir, "pdf")
    fun state(c: Context) = File(c.filesDir, "sync-state.json")
}

fun githubSync(c: Context): GitHubSync? {
    val repo = c.getSharedPreferences("poet", Context.MODE_PRIVATE).getString("repo", null) ?: return null
    val token = Secret.load(c) ?: return null
    return GitHubSync(repo, token, Paths.poems(c), Paths.pdfs(c), Paths.state(c))
}

/** A sync every 15 minutes while there is a network, so the phone stays current when closed. */
class SyncWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val sync = githubSync(applicationContext) ?: return Result.success()
        return runCatching { SyncLock.withLock { sync.sync() } }
            .fold({ Result.success() }, { Result.retry() })
    }

    companion object {
        fun schedule(c: Context) {
            val req = PeriodicWorkRequestBuilder<SyncWorker>(15, TimeUnit.MINUTES)
                .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
                .build()
            WorkManager.getInstance(c).enqueueUniquePeriodicWork("sync", ExistingPeriodicWorkPolicy.KEEP, req)
        }
    }
}

/** One sync at a time, from the app or the background worker. */
object SyncLock {
    private val lock = kotlinx.coroutines.sync.Mutex()
    suspend fun <T> withLock(block: () -> T): T = lock.withLock { block() }
}
