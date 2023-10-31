/*
 * This Kotlin source file was generated by the Gradle 'init' task.
 */
package wordpress.rs

import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import uniffi.wordpress_api.combineStrings

class LibraryTest {
    @Before
    fun setup() {
    }

    @Test
    fun testAddCustom() {
        assertEquals(Library().addCustomFromRust(2, 4), 6)
    }

    @Test
    fun testCombineStrings() {
        assertEquals(combineStrings("this", "that"), "this-that")
    }
}