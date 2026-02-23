package rs.wordpress.example.shared.ui.settings

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import org.jetbrains.compose.ui.tooling.preview.Preview
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.example.shared.ui.components.LoadingIndicator

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun SiteSettingsScreen(
    apiClient: WpApiClient,
    viewModel: SiteSettingsViewModel = remember { SiteSettingsViewModel(apiClient) },
    onBackClicked: () -> Unit = {}
) {
    val settings by viewModel.settings.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Site Settings") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        if (isLoading) {
            LoadingIndicator(modifier = Modifier.padding(paddingValues))
        } else {
            LazyColumn(
                modifier = Modifier.fillMaxSize().padding(paddingValues)
            ) {
                settings?.let { s ->
                    item {
                        ListItem(
                            headlineContent = { Text(s.title) },
                            overlineContent = { Text("Title") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.description) },
                            overlineContent = { Text("Description") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.url) },
                            overlineContent = { Text("URL") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.email) },
                            overlineContent = { Text("Email") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.timezone.ifEmpty { "(not set)" }) },
                            overlineContent = { Text("Timezone") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.language) },
                            overlineContent = { Text("Language") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.dateFormat) },
                            overlineContent = { Text("Date Format") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.timeFormat) },
                            overlineContent = { Text("Time Format") }
                        )
                    }
                    item {
                        ListItem(
                            headlineContent = { Text(s.postsPerPage.toString()) },
                            overlineContent = { Text("Posts Per Page") }
                        )
                    }
                }
            }
        }
    }
}
