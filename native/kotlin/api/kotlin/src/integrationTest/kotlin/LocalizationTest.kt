package rs.wordpress.api.kotlin

import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ExampleLocalizableException
import uniffi.wp_api.UniffiLocalizable
import uniffi.wp_api.WpLocale

class LocalizationTest {
    @Test
    fun testFooError() = runTest {
        val foo = ExampleLocalizableException.Hello(value = "world")
        assertEquals(
            "Hello \u2068world\u2069!",
            UniffiLocalizable.exampleLocalizableError(foo).localize(locale = null)
        )
        assertEquals(
            "Merhaba \u2068world\u2069!",
            UniffiLocalizable.exampleLocalizableError(foo).localize(locale = WpLocale.TR_TR)
        )
    }
}
