package rs.wordpress.example.shared

import android.graphics.BitmapFactory
import android.os.Build
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId

class AndroidPlatform : Platform {
    override val name: String = "Android ${Build.VERSION.SDK_INT}"
}

actual fun getPlatform(): Platform = AndroidPlatform()

actual fun localTestSiteUrl() = object: TestSiteUrl {
    override val siteUrl: String
        get() = "http://10.0.2.2"
}

@OptIn(ExperimentalComposeUiApi::class)
actual fun Modifier.testTagsForUiTests(): Modifier = semantics { testTagsAsResourceId = true }

/** The widest a stored image is read at. A photo is read smaller than it is, to fit in memory. */
private const val STORED_IMAGE_MAX_WIDTH = 1080

actual fun loadStoredImage(path: String): ImageBitmap? {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeFile(path, bounds)
    var sample = 1
    while (bounds.outWidth / sample > STORED_IMAGE_MAX_WIDTH) {
        sample *= 2
    }
    val options = BitmapFactory.Options().apply { inSampleSize = sample }
    return BitmapFactory.decodeFile(path, options)?.asImageBitmap()
}
