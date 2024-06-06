package rs.wordpress.example

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.runBlocking
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import rs.wordpress.api.android.BuildConfig
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpRequestSuccess
import rs.wordpress.example.shared.App
import rs.wordpress.example.shared.di.commonModule
import uniffi.wp_api.UserWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class MainActivity : ComponentActivity() {
    private val siteUrl = BuildConfig.TEST_SITE_URL
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = BuildConfig.TEST_ADMIN_USERNAME,
        password = BuildConfig.TEST_ADMIN_PASSWORD
    )
    private val client = WpApiClient(siteUrl, authentication)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        startKoin {
            androidContext(this@MainActivity)
            modules(commonModule())
        }

        setContent {
            App()
            //RsApp(::fetchUsers)
        }
    }

    private fun fetchUsers(): List<UserWithEditContext> {
        val usersResult = runBlocking {
            client.request { requestBuilder ->
                requestBuilder.users().listWithEditContext(params = null)
            }
        }
        return when (usersResult) {
            is WpRequestSuccess -> usersResult.data
            else -> listOf()
        }
    }
}

@Preview
@Composable
fun AppAndroidPreview() {
    RsApp { listOf() }
}

@Composable
@org.jetbrains.compose.ui.tooling.preview.Preview
fun RsApp(fetchUsers: () -> List<UserWithEditContext>) {
    MaterialTheme {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize(),
        ) {
            var showContent by remember { mutableStateOf(false) }
            if (!showContent) {
                Button(onClick = { showContent = !showContent }) {
                    Text("Fetch Users")
                }
            }
            AnimatedVisibility(showContent) {
                LazyColumn {
                    items(fetchUsers()) {
                        UserCard2(it)
                    }
                }
            }
        }
    }
}

@Composable
fun UserCard2(user: UserWithEditContext) {
    Row(modifier = Modifier.padding(all = 8.dp)) {
        Column {
            Text(
                text = user.name,
                color = MaterialTheme.colorScheme.secondary,
                style = MaterialTheme.typography.titleSmall
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text =  user.email,
                color = MaterialTheme.colorScheme.secondary,
                style = MaterialTheme.typography.titleSmall
            )
        }
    }
}