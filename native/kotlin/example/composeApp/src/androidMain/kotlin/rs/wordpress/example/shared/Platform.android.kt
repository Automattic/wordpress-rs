package rs.wordpress.example.shared

import android.os.Build
import rs.wordpress.api.kotlin.WpApiClient
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class AndroidPlatform : Platform {
    override val name: String = "Android ${Build.VERSION.SDK_INT}"
}

actual fun getPlatform(): Platform = AndroidPlatform()

actual fun createWpApiClient() =
    WpApiClient(
        siteUrl = "http://10.0.2.2",
        authentication = wpAuthenticationFromUsernameAndPassword(
            "test@example.com",
            password = "WpXcVrSWZvPcI1gD9muIOF8l"
        )
    )