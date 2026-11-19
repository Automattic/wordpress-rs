package rs.wordpress.example.shared

import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.toComposeImageBitmap
import org.jetbrains.skia.Image
import java.io.File

class JVMPlatform: Platform {
    override val name: String = "Java ${System.getProperty("java.version")}"
}

actual fun getPlatform(): Platform = JVMPlatform()

actual fun localTestSiteUrl() = object: TestSiteUrl {
    override val siteUrl: String
        get() = "http://localhost"
}

actual fun Modifier.testTagsForUiTests(): Modifier = this

// Skia throws for a file it can't read as an image, which is what `null` is for.
@Suppress("TooGenericExceptionCaught", "SwallowedException")
actual fun loadStoredImage(path: String): ImageBitmap? = try {
    Image.makeFromEncoded(File(path).readBytes()).toComposeImageBitmap()
} catch (e: Exception) {
    null
}
