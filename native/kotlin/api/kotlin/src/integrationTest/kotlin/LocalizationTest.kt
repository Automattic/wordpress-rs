package rs.wordpress.api.kotlin

import kotlin.test.assertEquals
import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.FooException
import uniffi.wp_api.UniffiLocalizable

class LocalizationTest {
    @Test
    fun testFooError() = runTest {
        val foo = FooException.Bar()
        assertEquals("Foo is bar", UniffiLocalizable.fooError(foo).localize(locale = null))
    }
}
