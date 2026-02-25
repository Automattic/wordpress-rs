package rs.wordpress.api.android

import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.security.keystore.StrongBoxUnavailableException
import android.util.Base64
import uniffi.wp_mobile.PasswordTransformer
import uniffi.wp_mobile.PasswordTransformerException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

/**
 * A [PasswordTransformer] implementation that uses the Android Keystore for
 * hardware-backed encryption.
 *
 * The AES key is generated and stored inside the Android Keystore. On physical
 * devices the key is held in secure hardware (TEE or StrongBox) and never
 * exposed to the application process. On emulators or devices without a
 * hardware-backed Keystore the key is protected in software only. Check
 * [isHardwareBacked] to determine which mode is active.
 *
 * Keys are addressed by application name. If a key with the given name already
 * exists in the Keystore it is loaded; otherwise a new one is created.
 *
 * ## Usage
 *
 * ```kotlin
 * val transformer = KeystorePasswordTransformer("my-app")
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
class KeystorePasswordTransformer(applicationName: String) : PasswordTransformer {

    private val key: SecretKey

    /**
     * Whether the encryption key is backed by secure hardware (TEE or StrongBox).
     *
     * Returns `false` on emulators or devices without a hardware-backed Keystore,
     * where the key is protected in software only.
     */
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

        key = (keyStore.getKey(applicationName, null) as? SecretKey)
            ?: generateKey(applicationName)
    }

    // The Cipher and KeyStore APIs throw a wide variety of checked and unchecked
    // exceptions (InvalidKeyException, BadPaddingException, IllegalBlockSizeException,
    // etc.). We catch broadly and wrap them into a single PasswordTransformerException
    // so callers get a uniform error type that crosses the UniFFI boundary cleanly.
    @Suppress("TooGenericExceptionCaught", "SwallowedException")
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
        } catch (e: PasswordTransformerException) {
            // Defensive: nothing in the try block throws this today, but if
            // future changes add early validation (as decrypt() does), this
            // prevents the broad catch below from double-wrapping it.
            throw e
        } catch (e: Exception) {
            val reason = "${e.javaClass.simpleName}: " +
                (e.message ?: "Unknown encryption error")
            throw PasswordTransformerException.EncryptionFailed(reason)
        }
    }

    @Suppress("TooGenericExceptionCaught", "SwallowedException") // See comment on encrypt()
    override fun decrypt(password: String): String {
        try {
            if (!password.matches(BASE64_PATTERN)) {
                throw PasswordTransformerException.DecryptionFailed(
                    "Ciphertext is not valid base64"
                )
            }

            val combined = Base64.decode(password, Base64.NO_WRAP)

            require(combined.size >= GCM_IV_SIZE + GCM_TAG_SIZE) { "Ciphertext too short" }

            val iv = combined.copyOfRange(0, GCM_IV_SIZE)
            val ciphertext = combined.copyOfRange(GCM_IV_SIZE, combined.size)

            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_SIZE_BITS, iv))

            val plaintext = cipher.doFinal(ciphertext)
            return String(plaintext, Charsets.UTF_8)
        } catch (e: PasswordTransformerException) {
            // Re-throw without wrapping — the base64 validation above throws
            // DecryptionFailed directly, and the broad catch below would otherwise
            // catch and double-wrap it (since PasswordTransformerException is a
            // subclass of Exception).
            throw e
        } catch (e: Exception) {
            val reason = "${e.javaClass.simpleName}: " +
                (e.message ?: "Unknown decryption error")
            throw PasswordTransformerException.DecryptionFailed(reason)
        }
    }

    companion object {
        private val BASE64_PATTERN = Regex("^[A-Za-z0-9+/]*=*$")
        private const val KEYSTORE_PROVIDER = "AndroidKeyStore"
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val GCM_IV_SIZE = 12
        private const val GCM_TAG_SIZE_BITS = 128
        private const val GCM_TAG_SIZE = GCM_TAG_SIZE_BITS / 8
        private const val AES_KEY_SIZE = 256

        private fun generateKey(applicationName: String): SecretKey {
            val builder = KeyGenParameterSpec.Builder(
                applicationName,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(AES_KEY_SIZE)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                builder.setIsStrongBoxBacked(true)
            }

            val keyGenerator = KeyGenerator.getInstance(
                KeyProperties.KEY_ALGORITHM_AES,
                KEYSTORE_PROVIDER
            )

            try {
                keyGenerator.init(builder.build())
                return keyGenerator.generateKey()
            } catch (_: StrongBoxUnavailableException) {
                // StrongBox not available — rebuild the spec without it.
                builder.setIsStrongBoxBacked(false)
                keyGenerator.init(builder.build())
                return keyGenerator.generateKey()
            }
        }
    }
}
