package com.d4shwood.poet.sync

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * The GitHub token, encrypted with a key that never leaves the Android
 * Keystore. Only the ciphertext is kept in the app's preferences.
 */
object Secret {
    private const val ALIAS = "poet-github-token"
    private const val PREF = "github_token"

    private fun key(): SecretKey {
        val ks = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (ks.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        gen.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .build(),
        )
        return gen.generateKey()
    }

    fun save(context: Context, token: String) {
        val c = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val sealed = c.iv + c.doFinal(token.trim().toByteArray())
        prefs(context).edit().putString(PREF, Base64.encodeToString(sealed, Base64.NO_WRAP)).apply()
    }

    fun load(context: Context): String? = runCatching {
        val sealed = Base64.decode(prefs(context).getString(PREF, null) ?: return null, Base64.NO_WRAP)
        val c = Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, sealed, 0, 12))
        c.doFinal(sealed, 12, sealed.size - 12).decodeToString()
    }.getOrNull()

    fun clear(context: Context) = prefs(context).edit().remove(PREF).apply()

    private fun prefs(context: Context) = context.getSharedPreferences("poet", Context.MODE_PRIVATE)
}
