package rs.wordpress.example.shared

import android.os.Build

class AndroidPlatform : Platform {
    override val name: String = "Android ${Build.VERSION.SDK_INT}"
}

actual fun getPlatform(): Platform = AndroidPlatform()

actual fun localTestSiteUrl() = object: TestSiteUrl {
    override val siteUrl: String
        get() = "http://10.0.2.2"
}