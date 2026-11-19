package rs.wordpress.example.shared

import androidx.compose.ui.Modifier

class JVMPlatform: Platform {
    override val name: String = "Java ${System.getProperty("java.version")}"
}

actual fun getPlatform(): Platform = JVMPlatform()

actual fun localTestSiteUrl() = object: TestSiteUrl {
    override val siteUrl: String
        get() = "http://localhost"
}

actual fun Modifier.testTagsForUiTests(): Modifier = this
