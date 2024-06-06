package rs.wordpress.example

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import org.koin.compose.KoinApplication
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.di.commonModule

fun main() = application {
    Window(
        onCloseRequest = ::exitApplication,
        title = "WordPressRsExample",
    ) {
        KoinApplication(application = {
            modules(commonModule())
        }) {
            App()
        }
    }
}
