package rs.wordpress.example.shared.ui.menulocations

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
fun MenuLocationListScreen(
    apiClient: WpApiClient,
    viewModel: MenuLocationListViewModel = remember { MenuLocationListViewModel(apiClient) },
    onBackClicked: () -> Unit = {}
) {
    val menuLocations by viewModel.menuLocations.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Menu Locations") },
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
                items(menuLocations) { menuLocation ->
                    ListItem(
                        headlineContent = { Text(menuLocation.name) },
                        supportingContent = { Text(menuLocation.description) }
                    )
                }
            }
        }
    }
}
