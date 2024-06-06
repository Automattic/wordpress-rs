package rs.wordpress.example.shared

import rs.wordpress.api.kotlin.WpApiClient
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class JVMPlatform: Platform {
    override val name: String = "Java ${System.getProperty("java.version")}"
}

actual fun getPlatform(): Platform = JVMPlatform()

actual fun createWpApiClient() =
    WpApiClient(
        siteUrl = "http://localhost",
        authentication = wpAuthenticationFromUsernameAndPassword(
            "test@example.com",
            password = "WpXcVrSWZvPcI1gD9muIOF8l"
        )
    )