package rs.wordpress.example.ui.welcome

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen

class WelcomeActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            WelcomeScreen()
        }
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    WelcomeScreen()
}
