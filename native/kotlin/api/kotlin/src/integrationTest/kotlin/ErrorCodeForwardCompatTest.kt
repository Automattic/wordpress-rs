package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostType
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class ErrorCodeForwardCompatTest {
    private val client = defaultApiClient()

    @Test
    fun testKnownErrorCodeHasValueAndRaw() = runTest {
        // Trigger a known error: retrieving a non-existent post type
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Custom("nonexistent_type"))
        }

        val errorCodeValue = result.wpErrorCodeValue()
        // `value` should be the known enum variant
        assertEquals(WpErrorCode.TYPE_INVALID, errorCodeValue.value)
        // `raw` should always contain the original API string
        assertEquals("rest_type_invalid", errorCodeValue.raw)
    }

    @Test
    fun testWpErrorCodeHelperReturnsKnownVariant() = runTest {
        // The wpErrorCode() helper extracts .value for convenience
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Custom("nonexistent_type"))
        }

        // This is the migration path: use == instead of `is` for enum comparison
        assertEquals(WpErrorCode.TYPE_INVALID, result.wpErrorCode())
    }

    @Test
    fun testRawStringIsAlwaysAvailable() = runTest {
        // Even for known error codes, the raw string is preserved.
        // This is the forward-compat guarantee: if a client was written before
        // a variant existed, they could check the raw string. After the variant
        // is added, their raw string check still works.
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Custom("nonexistent_type"))
        }

        val errorCodeValue = result.wpErrorCodeValue()
        assertNotNull(errorCodeValue.value, "Known error codes should have a value")
        assertEquals("rest_type_invalid", errorCodeValue.raw)
    }
}
