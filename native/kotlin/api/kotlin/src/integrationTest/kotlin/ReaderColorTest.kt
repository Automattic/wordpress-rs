package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import uniffi.wp_mobile.ReaderColor
import uniffi.wp_mobile.readerColorFromArgb
import uniffi.wp_mobile.readerColorFromComponents
import uniffi.wp_mobile.readerColorFromHex
import kotlin.test.assertEquals
import kotlin.test.assertNull

class ReaderColorTest {
    private val translucentBlue = ReaderColor(red = 0x38u, green = 0x58u, blue = 0xe9u, alpha = 0x80u)

    @Test
    fun readsAndWritesCssHex() {
        assertEquals(translucentBlue, readerColorFromHex("#3858e980"))
        assertEquals("#3858e980", translucentBlue.hex())
        assertNull(readerColorFromHex("blue"))
    }

    @Test
    fun readsAndWritesAndroidColorInts() {
        // Android's colour ints are signed, so a colour with its top bit set is negative.
        val colorInt = 0x803858e9.toInt()
        assertEquals(colorInt, translucentBlue.argb())
        assertEquals(translucentBlue, readerColorFromArgb(colorInt))
    }

    @Test
    fun readsComponentsBetween0And1() {
        val color = readerColorFromComponents(red = 0.0, green = 1.0, blue = 0.2, alpha = 1.5)
        assertEquals(ReaderColor(red = 0u, green = 255u, blue = 51u, alpha = 255u), color)
    }
}
