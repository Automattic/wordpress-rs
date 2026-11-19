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

// The desktop app has no web view, so it names the system and Java itself.
actual fun readerPageViewUserAgent(): String {
    val system = "${System.getProperty("os.name")} ${System.getProperty("os.version")}"
    return "Mozilla/5.0 ($system) Java/${System.getProperty("java.version")} $READER_PAGE_VIEW_APP_NAME"
}

// Skia throws for a file it can't read as an image, which is what `null` is for.
@Suppress("TooGenericExceptionCaught", "SwallowedException")
actual fun loadStoredImage(path: String): ImageBitmap? = try {
    Image.makeFromEncoded(File(path).readBytes()).toComposeImageBitmap()
} catch (e: Exception) {
    null
}
