package rs.wordpress.api.android

import android.graphics.Color
import android.graphics.ColorSpace
import org.junit.Test
import uniffi.wp_mobile.ReaderColor
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue

class ReaderColorExtensionsTest {
    private val translucentBlue = ReaderColor(red = 0x38u, green = 0x58u, blue = 0xe9u, alpha = 0x80u)

    @Test
    fun readsAndWritesColorInts() {
        val colorInt = Color.argb(0x80, 0x38, 0x58, 0xe9)
        assertEquals(translucentBlue, ReaderColor.fromColorInt(colorInt))
        assertEquals(colorInt, translucentBlue.toColorInt())
    }

    @Test
    fun readsAndWritesColors() {
        assertEquals(translucentBlue, ReaderColor.fromColor(translucentBlue.toColor()))
        assertEquals("#3858e980", ReaderColor.fromColor(Color.valueOf(0x803858e9.toInt())).hex())
    }

    @Test
    fun convertsOtherColorSpacesToSrgb() {
        // Display P3's components don't mean the same as sRGB's, so reading them as sRGB would be
        // wrong.
        val p3Color = Color.valueOf(0.2f, 0.6f, 0.3f, 1f, ColorSpace.get(ColorSpace.Named.DISPLAY_P3))
        val color = ReaderColor.fromColor(p3Color)
        assertNotEquals(ReaderColor(red = 51u, green = 153u, blue = 77u, alpha = 255u), color)
        assertTrue(color.green > color.red && color.green > color.blue)
    }
}
