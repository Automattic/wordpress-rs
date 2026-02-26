package rs.wordpress.example

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import org.koin.compose.KoinApplication
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.di.commonModules
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_mobile.AesGcmPasswordTransformer

fun main() {
    // Load native libraries before initializing the app
    NativeLibraryLoader.loadLibraries()

    val homeDir = System.getProperty("user.home")
    val accountRepository = AccountRepository(
        rootPath = "$homeDir/.wordpress-rs/accounts",
        passwordTransformer = AesGcmPasswordTransformer("wordpress-rs-desktop-example")
    )

    application {
        Window(
            onCloseRequest = ::exitApplication,
            title = "WordPressRsExample",
        ) {
            KoinApplication(application = {
                modules(commonModules(accountRepository))
            }) {
                // Authentication is not supported on Desktop
                App(authenticationEnabled = false, authenticateSite = {}, authenticateWpCom = null)
            }
        }
    }
}
