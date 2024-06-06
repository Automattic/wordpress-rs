package rs.wordpress.example.ui.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier

@Composable
@org.jetbrains.compose.ui.tooling.preview.Preview
fun LoginScreen(authenticateSite: (String) -> Unit) {
    MaterialTheme {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize(),
        ) {
            var siteUrl by remember { mutableStateOf("https://loudly-special.jurassic.ninja/") }
            TextField(value = siteUrl, onValueChange = { siteUrl = it })
            Button(onClick = { authenticateSite(siteUrl) }) {
                Text("Login")
            }
        }
    }
}
