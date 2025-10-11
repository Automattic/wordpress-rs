package rs.wordpress.example

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import org.koin.compose.KoinApplication
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.di.commonModules

fun main() {
    // Load native libraries before initializing the app
    NativeLibraryLoader.loadLibraries()

    application {
        Window(
            onCloseRequest = ::exitApplication,
            title = "WordPressRsExample",
        ) {
            KoinApplication(application = {
                modules(commonModules())
            }) {
                // Authentication is not supported on Desktop
                App(authenticationEnabled = false, authenticateSite = {})
            }
        }
    }
}
