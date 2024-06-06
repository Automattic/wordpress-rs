package rs.wordpress.example.shared.ui.welcome

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import org.koin.compose.KoinContext
import org.koin.compose.koinInject
import rs.wordpress.example.shared.domain.AuthenticatedSite

@Composable
@org.jetbrains.compose.ui.tooling.preview.Preview
fun WelcomeScreen(onLoginClicked: () -> Unit, onSiteClicked: (AuthenticatedSite) -> Unit) {
    KoinContext{
        val welcomeViewModel = koinInject<WelcomeViewModel>()

        MaterialTheme {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.Center,
                modifier = Modifier.fillMaxSize(),
            ) {
                LazyColumn {
                    items(welcomeViewModel.getSiteList()) { site ->
                        Button(onClick = { onSiteClicked(site) }) {
                            Text(site.name)
                            Modifier.background(Color.Yellow)
                        }
                    }
                }
            }
        }
    }
}
