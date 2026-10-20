package com.d4shwood.poet.sync

import org.junit.Assert.assertEquals
import org.junit.Test

class PlanTest {
    @Test
    fun blobShaMatchesGit() {
        // From `printf … | git hash-object --stdin`.
        assertEquals("14273fdaee3c8ba81f386dfacb1dff44981a37cc", blobSha("Liman\n".toByteArray()))
        assertEquals("3b35e7c7bb497fea074e9b11f7711439a4efcc8f", blobSha("Şiir ğüı\n".toByteArray()))
        assertEquals("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391", blobSha(ByteArray(0)))
    }

    @Test
    fun threeWay() {
        val base = mapOf("same" to "a", "theirs" to "a", "mine" to "a", "both" to "a", "gone-there" to "a", "gone-here" to "a")
        val local = mapOf("same" to "a", "theirs" to "a", "mine" to "b", "both" to "b", "gone-there" to "a", "new-here" to "n")
        val remote = mapOf("same" to "a", "theirs" to "c", "mine" to "a", "both" to "c", "gone-here" to "a", "new-there" to "m")
        assertEquals(
            listOf(
                Step.Conflict("both", "c"),
                Step.DeleteRemote("gone-here"),
                Step.DeleteLocal("gone-there"),
                Step.Upload("mine"),
                Step.Upload("new-here"),
                Step.Download("new-there", "m"),
                Step.Download("theirs", "c"),
            ),
            plan(local, remote, base),
        )
    }

    @Test
    fun firstSyncDownloadsEverything() {
        assertEquals(listOf(Step.Download("Liman", "x")), plan(emptyMap(), mapOf("Liman" to "x"), emptyMap()))
    }

    @Test
    fun conflictNames() {
        val taken = setOf("Liman (telefon)")
        assertEquals("Liman (telefon 2)", conflictName("Liman") { it in taken })
    }
}
