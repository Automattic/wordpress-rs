package rs.wordpress.example.shared

import android.os.Build
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
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
