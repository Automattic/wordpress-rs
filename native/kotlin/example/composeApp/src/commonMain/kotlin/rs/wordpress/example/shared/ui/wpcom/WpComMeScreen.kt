package rs.wordpress.example.shared.ui.wpcom

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import org.jetbrains.compose.ui.tooling.preview.Preview
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.WpComUserInfo
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun WpComMeScreen(
    wpComApiClient: WpComApiClient,
    onBackClicked: () -> Unit = {}
) {
    var userInfo by remember { mutableStateOf<WpComUserInfo?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }

    LaunchedEffect(Unit) {
        when (val result = wpComApiClient.request { it.me().get() }) {
            is WpRequestResult.Success -> {
                userInfo = result.response.data
                isLoading = false
            }
            else -> {
                error = result.toString()
                isLoading = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Me") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        when {
            isLoading -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(paddingValues),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            }
            error != null -> {
                Box(
                    modifier = Modifier.fillMaxSize().padding(paddingValues),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        text = "Error: $error",
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
            userInfo != null -> {
                val info = userInfo!!
                LazyColumn(
                    modifier = Modifier.fillMaxSize().padding(paddingValues)
                ) {
                    item {
                        ListItem(
                            headlineContent = { Text("Display Name") },
                            supportingContent = { Text(info.displayName) }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text("Username") },
                            supportingContent = { Text(info.username) }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text("Email") },
                            supportingContent = { Text(info.email) }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text("Site Count") },
                            supportingContent = { Text(info.siteCount.toString()) }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text("Created Date") },
                            supportingContent = { Text(formatDate(info.creationDate)) }
                        )
                    }
                }
            }
        }
    }
}

private fun formatDate(date: Date): String {
    return try {
        SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault()).format(date)
    } catch (e: Exception) {
        date.toString()
    }
}
