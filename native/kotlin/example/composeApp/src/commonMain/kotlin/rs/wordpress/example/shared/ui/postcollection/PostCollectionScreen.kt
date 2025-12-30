package rs.wordpress.example.shared.ui.postcollection

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material.Button
import androidx.compose.material.Card
import androidx.compose.material.CircularProgressIndicator
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.material.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import org.koin.compose.koinInject
import rs.wordpress.example.shared.ui.components.PostCard

@Composable
@Preview
fun PostCollectionScreen(
    viewModel: PostCollectionViewModel = koinInject(),
    onBackClicked: (() -> Unit)? = null
) {
    val state by viewModel.state.collectAsState()
    val posts by viewModel.posts.collectAsState()
    val listState = rememberLazyListState()

    MaterialTheme {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.fillMaxSize().padding(16.dp),
        ) {
            // Back button (for desktop)
            if (onBackClicked != null) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Start
                ) {
                    TextButton(onClick = onBackClicked) {
                        Text("← Back")
                    }
                }
                Spacer(modifier = Modifier.height(8.dp))
            }

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

@Composable
fun FilterControls(
    currentFilter: String?,
    onFilterChange: (String?) -> Unit
) {
    val currentFilterStr = currentFilter ?: ""
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = 2.dp
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Filter",
                style = MaterialTheme.typography.subtitle1,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(8.dp))
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                FilterButton(
                    text = "All",
                    isSelected = currentFilter == null,
                    onClick = { onFilterChange(null) }
                )
                FilterButton(
                    text = "Drafts",
                    isSelected = currentFilterStr.contains("draft", ignoreCase = true),
                    onClick = { onFilterChange("draft") }
                )
                FilterButton(
                    text = "Published",
                    isSelected = currentFilterStr.contains("publish", ignoreCase = true),
                    onClick = { onFilterChange("publish") }
                )
            }
        }
    }
}

@Composable
fun RowScope.FilterButton(
    text: String,
    isSelected: Boolean,
    onClick: () -> Unit
) {
    if (isSelected) {
        Button(
            onClick = onClick,
            modifier = Modifier.weight(1f)
        ) {
            Text(text)
        }
    } else {
        TextButton(
            onClick = onClick,
            modifier = Modifier.weight(1f)
        ) {
            Text(text)
        }
    }
}

@Composable
fun InfoCard(
    state: CollectionState,
    postCount: Int
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = 2.dp
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = "Collection Info",
                style = MaterialTheme.typography.subtitle1,
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
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = 4.dp
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
                        style = MaterialTheme.typography.subtitle2,
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
                                text = "✓ No more pages",
                                style = MaterialTheme.typography.caption,
                                color = MaterialTheme.colors.primary
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
                        style = MaterialTheme.typography.subtitle2,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colors.error
                    )
                    Spacer(modifier = Modifier.height(4.dp))
                    Text(
                        text = formatFetchError(lastError),
                        style = MaterialTheme.typography.caption,
                        color = MaterialTheme.colors.error
                    )
                }
            }
        }
    }
}

fun formatFetchError(error: Any): String {
    return "Error: $error"
}
