package rs.wordpress.api.android

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import uniffi.wp_mobile.PasswordTransformer
import uniffi.wp_mobile.PasswordTransformerException

/**
 * A [PasswordTransformer] implementation that uses the Android Keystore for
 * hardware-backed AES-256-GCM encryption.
 *
 * The AES key is generated and stored inside the Android Keystore — it never
 * leaves the secure hardware (TEE or StrongBox). On devices that support
 * StrongBox (API 28+), the key is stored in the dedicated secure element for
 * stronger isolation. Check [isHardwareBacked] to determine the security level.
 *
 * Keys are addressed by alias. If a key with the given alias already exists in
 * the Keystore it is loaded; otherwise a new one is created. Use distinct
 * aliases to maintain separate keys per account or context.
 *
 * ## Usage
 *
 * ```kotlin
 * // Create a transformer with the default alias
 * val transformer = KeystorePasswordTransformer()
 *
 * // Or use a custom alias for per-account keys
 * val transformer = KeystorePasswordTransformer("account-42")
 *
 * // Encrypt a password
 * val encrypted: String = transformer.encrypt("hunter2")
 *
 * // Decrypt it back
 * val decrypted: String = transformer.decrypt(encrypted)
 * assert(decrypted == "hunter2")
 *
 * // Check hardware backing
 * if (transformer.isHardwareBacked) {
 *     // Key is in TEE or StrongBox
 * }
 * ```
 */
class KeystorePasswordTransformer(alias: String = DEFAULT_ALIAS) : PasswordTransformer {

    private val key: SecretKey

    val isHardwareBacked: Boolean
        get() {
            val factory = SecretKeyFactory.getInstance(key.algorithm, KEYSTORE_PROVIDER)
            val keyInfo = factory.getKeySpec(key, KeyInfo::class.java) as KeyInfo
            return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                keyInfo.securityLevel >= KeyProperties.SECURITY_LEVEL_TRUSTED_ENVIRONMENT
            } else {
                @Suppress("DEPRECATION")
                keyInfo.isInsideSecureHardware
            }
        }

    init {
        val keyStore = KeyStore.getInstance(KEYSTORE_PROVIDER)
        keyStore.load(null)

        key = if (keyStore.containsAlias(alias)) {
            keyStore.getKey(alias, null) as SecretKey
        } else {
            generateKey(alias)
        }
    }

    override fun encrypt(password: String): String {
        try {
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.ENCRYPT_MODE, key)

            val iv = cipher.iv
            val ciphertext = cipher.doFinal(password.toByteArray(Charsets.UTF_8))

            val combined = ByteArray(iv.size + ciphertext.size)
            System.arraycopy(iv, 0, combined, 0, iv.size)
            System.arraycopy(ciphertext, 0, combined, iv.size, ciphertext.size)

            return Base64.encodeToString(combined, Base64.NO_WRAP)
        } catch (e: Exception) {
            throw PasswordTransformerException.EncryptionFailed(e.message ?: "Unknown encryption error")
        }
    }

    override fun decrypt(password: String): String {
        try {
            val combined = Base64.decode(password, Base64.NO_WRAP)

            if (combined.size < GCM_IV_SIZE + GCM_TAG_SIZE) {
                throw IllegalArgumentException("Ciphertext too short")
            }

            val iv = combined.copyOfRange(0, GCM_IV_SIZE)
            val ciphertext = combined.copyOfRange(GCM_IV_SIZE, combined.size)

            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_SIZE_BITS, iv))

            val plaintext = cipher.doFinal(ciphertext)
            return String(plaintext, Charsets.UTF_8)
        } catch (e: PasswordTransformerException) {
            throw e
        } catch (e: Exception) {
            throw PasswordTransformerException.DecryptionFailed(e.message ?: "Unknown decryption error")
        }
    }

    companion object {
        private const val DEFAULT_ALIAS = "wordpress-rs-password-transformer"
        private const val KEYSTORE_PROVIDER = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_SIZE = 12
        private const val GCM_TAG_SIZE_BITS = 128
        private const val GCM_TAG_SIZE = GCM_TAG_SIZE_BITS / 8

        private fun generateKey(alias: String): SecretKey {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                try {
                    return generateKeyWithSpec(alias, strongBox = true)
                } catch (_: StrongBoxUnavailableException) {
                    // StrongBox not available on this device, fall back to TEE
                }
            }
            return generateKeyWithSpec(alias, strongBox = false)
        }

        private fun generateKeyWithSpec(alias: String, strongBox: Boolean): SecretKey {
            val builder = KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)

            if (strongBox && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                builder.setIsStrongBoxBacked(true)
            }

            val keyGenerator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES,
                KEYSTORE_PROVIDER
            )
            keyGenerator.init(builder.build())
            return keyGenerator.generateKey()
        }
    }
}
