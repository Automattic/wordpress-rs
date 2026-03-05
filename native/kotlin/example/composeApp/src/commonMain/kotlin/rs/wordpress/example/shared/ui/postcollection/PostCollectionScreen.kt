package rs.wordpress.example.shared.ui.postcollection

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import androidx.compose.runtime.remember
import rs.wordpress.example.shared.ui.components.PostCard
import uniffi.wp_mobile.WpService

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun PostCollectionScreen(
    wpService: WpService,
    viewModel: PostCollectionViewModel = remember { PostCollectionViewModel(wpService) },
    onBackClicked: (() -> Unit)? = null
) {
    val state by viewModel.state.collectAsState()
    val posts by viewModel.posts.collectAsState()
    val listState = rememberLazyListState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Post Collection") },
                navigationIcon = {
                    if (onBackClicked != null) {
                        IconButton(onClick = onBackClicked) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.fillMaxSize().padding(paddingValues).padding(horizontal = 16.dp),
        ) {
            // Filter controls
            FilterControls(
                currentFilter = state.filterStatusString,
                onFilterChange = { viewModel.setFilter(it) }
            )

            Spacer(modifier = Modifier.height(16.dp))

            // Info card
            InfoCard(
                state = state,
                postCount = posts.size
            )

            Spacer(modifier = Modifier.height(16.dp))

            // Posts list with fetch card at the end
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxSize(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                // Post items
                items(posts) { post ->
                    PostCard(post)
                }

                // Fetch next page card at the end
                item {
                    FetchNextPageCard(
                        state = state,
                        onFetchClick = { viewModel.fetchNextPage() }
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FilterControls(
    currentFilter: String?,
    onFilterChange: (String?) -> Unit
) {
    val currentFilterStr = currentFilter ?: ""
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Filter",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                FilterChip(
                    selected = currentFilter == null,
                    onClick = { onFilterChange(null) },
                    label = { Text("All") }
                )
                FilterChip(
                    selected = currentFilterStr.contains("draft", ignoreCase = true),
                    onClick = { onFilterChange("draft") },
                    label = { Text("Drafts") }
                )
                FilterChip(
                    selected = currentFilterStr.contains("publish", ignoreCase = true),
                    onClick = { onFilterChange("publish") },
                    label = { Text("Published") }
                )
            }
        }
    }
}

@Composable
fun InfoCard(
    state: CollectionState,
    postCount: Int
) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Collection Info",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(text = "Current Filter: ${state.filterDisplayName}")
            Text(text = "Posts in Cache: $postCount")
            Text(text = "Pages Fetched: ${state.currentPage}")
        }
    }
}

@Composable
fun FetchNextPageCard(
    state: CollectionState,
    onFetchClick: () -> Unit
) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            if (state.isFetching) {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text("Fetching page ${state.nextPage}...")
            } else {
                // Check if there are more pages to fetch
                val lastResult = state.lastFetchResult
                val hasMorePages = lastResult?.totalPages?.let { totalPages ->
                    state.currentPage.toUInt() < totalPages
                } ?: true // Show button if we haven't fetched any pages yet

                // Show fetch button only if there are more pages
                if (hasMorePages) {
                    Button(
                        onClick = onFetchClick,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Fetch Page ${state.nextPage}")
                    }
                }

                // Show last fetch result if available
                if (lastResult != null) {
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Last Fetch",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text("Page ${lastResult.currentPage} • ${lastResult.entityIds.size} posts")
                    val totalItems = lastResult.totalItems
                    if (totalItems != null) {
                        Text("Total items: $totalItems")
                    }
                    val totalPages = lastResult.totalPages
                    if (totalPages != null) {
                        Text("Total pages: $totalPages")
                        if (lastResult.currentPage >= totalPages) {
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = "No more pages",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                    }
                }

                // Show error if fetch failed
                val lastError = state.lastFetchError
                if (lastError != null) {
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = "Fetch Error",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.error
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = formatFetchError(lastError),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error
                    )
                }
            }
        }
    }
}

fun formatFetchError(error: Any): String {
    return "Error: $error"
}
