package rs.wordpress.api.android

import android.util.Base64
import org.junit.After
import org.junit.Test
import uniffi.wp_mobile.PasswordTransformerException
import java.security.KeyStore
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class KeystorePasswordTransformerTest {

    private val testAliases = mutableListOf<String>()

    private fun createTransformer(alias: String = "test-key-${System.nanoTime()}"): KeystorePasswordTransformer {
        testAliases.add(alias)
        return KeystorePasswordTransformer(alias)
    }

    @After
    fun tearDown() {
        val keyStore = KeyStore.getInstance("AndroidKeyStore")
        keyStore.load(null)
        for (alias in testAliases) {
            if (keyStore.containsAlias(alias)) {
                keyStore.deleteEntry(alias)
            }
        }
    }

    @Test
    fun roundTripEncryptDecrypt() {
        val transformer = createTransformer()
        val plaintext = "my-secret-password"

        val encrypted = transformer.encrypt(plaintext)
        val decrypted = transformer.decrypt(encrypted)

        assertEquals(plaintext, decrypted)
    }

    @Test
    fun encryptedOutputDiffersFromPlaintext() {
        val transformer = createTransformer()
        val plaintext = "my-secret-password"

        val encrypted = transformer.encrypt(plaintext)

        assertNotEquals(plaintext, encrypted)
    }

    @Test
    fun samePlaintextProducesDifferentCiphertext() {
        val transformer = createTransformer()
        val plaintext = "my-secret-password"

        val encrypted1 = transformer.encrypt(plaintext)
        val encrypted2 = transformer.encrypt(plaintext)

        assertNotEquals(encrypted1, encrypted2)
    }

    @Test
    fun emptyStringRoundTrip() {
        val transformer = createTransformer()

        val encrypted = transformer.encrypt("")
        val decrypted = transformer.decrypt(encrypted)

        assertEquals("", decrypted)
    }

    @Test
    fun unicodeRoundTrip() {
        val transformer = createTransformer()
        val plaintext = "p\u00e4ssw\u00f6rd-\u2603-\ud83d\udd11-\u4f60\u597d"

        val encrypted = transformer.encrypt(plaintext)
        val decrypted = transformer.decrypt(encrypted)

        assertEquals(plaintext, decrypted)
    }

    @Test
    fun longPasswordRoundTrip() {
        val transformer = createTransformer()
        val plaintext = "a".repeat(10_000)

        val encrypted = transformer.encrypt(plaintext)
        val decrypted = transformer.decrypt(encrypted)

        assertEquals(plaintext, decrypted)
    }

    @Test
    fun wrongKeyAliasFailsToDecrypt() {
        val transformer1 = createTransformer()
        val transformer2 = createTransformer()

        val encrypted = transformer1.encrypt("secret")

        assertFailsWith<PasswordTransformerException.DecryptionFailed> {
            transformer2.decrypt(encrypted)
        }
    }

    @Test
    fun isHardwareBackedReturnsBooleanWithoutCrashing() {
        val transformer = createTransformer()

        // Just verify it returns without throwing — actual value depends on device
        val result = transformer.isHardwareBacked
        assertTrue(result || !result)
    }

    @Test
    fun reusingAliasLoadsExistingKey() {
        val alias = "reuse-test-${System.nanoTime()}"
        testAliases.add(alias)

        val transformer1 = KeystorePasswordTransformer(alias)
        val encrypted = transformer1.encrypt("secret")

        val transformer2 = KeystorePasswordTransformer(alias)
        val decrypted = transformer2.decrypt(encrypted)

        assertEquals("secret", decrypted)
    }

    @Test
    fun decryptInvalidBase64Fails() {
        val transformer = createTransformer()

        assertFailsWith<PasswordTransformerException.DecryptionFailed> {
            transformer.decrypt("not-valid-base64!!!")
        }
    }

    @Test
    fun decryptTruncatedCiphertextFails() {
        val transformer = createTransformer()

        val tooShort = Base64.encodeToString(ByteArray(10), Base64.NO_WRAP)

        assertFailsWith<PasswordTransformerException.DecryptionFailed> {
            transformer.decrypt(tooShort)
        }
    }
}
