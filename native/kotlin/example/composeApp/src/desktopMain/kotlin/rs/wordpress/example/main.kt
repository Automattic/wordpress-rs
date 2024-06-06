package rs.wordpress.example

import androidx.compose.ui.window.Window
import androidx.compose.ui.window.application
import org.koin.compose.KoinApplication
import rs.wordpress.example.shared.di.commonModules
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen

fun main() = application {
    Window(
        onCloseRequest = ::exitApplication,
        title = "WordPressRsExample",
    ) {
        KoinApplication(application = {
            modules(commonModules())
        }) {
            WelcomeScreen()
        }
    }
}
