package rs.wordpress.example.shared.ui.postmetadatacollection

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import androidx.compose.runtime.remember
import uniffi.wp_mobile.PostItemState
import uniffi.wp_mobile.WpService
import uniffi.wp_mobile_cache.ListState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun PostMetadataCollectionScreen(
    wpService: WpService,
    postTypeSlug: String = "post",
    viewModel: PostMetadataCollectionViewModel = remember {
        PostMetadataCollectionViewModel(wpService, postTypeSlug)
    },
    onBackClicked: (() -> Unit)? = null
) {
    val state by viewModel.state.collectAsState()
    val items by viewModel.items.collectAsState()
    val listState = rememberLazyListState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Post Metadata") },
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

            // Info card with refresh button
            InfoCard(
                state = state,
                itemCount = items.size,
                onRefreshClick = { viewModel.refresh() },
                postTypeSlug = postTypeSlug
            )

            Spacer(modifier = Modifier.height(16.dp))

            // Items list with load more card at the end
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxSize(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(items) { item ->
                    PostItemCard(item)
                }

                // Load next page card at the end
                item {
                    LoadNextPageCard(
                        state = state,
                        onLoadClick = { viewModel.loadNextPage() }
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
                    selected = currentFilter?.contains("draft", ignoreCase = true) == true,
                    onClick = { onFilterChange("draft") },
                    label = { Text("Drafts") }
                )
                FilterChip(
                    selected = currentFilter?.contains("publish", ignoreCase = true) == true,
                    onClick = { onFilterChange("publish") },
                    label = { Text("Published") }
                )
            }
        }
    }
}

@Composable
fun InfoCard(
    state: PostMetadataCollectionState,
    itemCount: Int,
    onRefreshClick: () -> Unit,
    postTypeSlug: String
) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column {
                    Text(
                        text = "Metadata Collection",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = "Post Type: ${postTypeSlug.replaceFirstChar { it.uppercase() }}",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
                Button(
                    onClick = onRefreshClick,
                    enabled = !state.isSyncing
                ) {
                    if (state.isSyncing) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(16.dp),
                            strokeWidth = 2.dp
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                    }
                    Text("Refresh")
                }
            }
            Spacer(modifier = Modifier.height(8.dp))
            Text(text = "Filter: ${state.filterDisplayName}")
            Text(text = "Items: $itemCount")
            Text(text = "Page: ${state.currentPage}" + (state.totalPages?.let { " / $it" } ?: ""))

            // Show sync state from database
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Text(text = "Sync State:")
                SyncStateIndicator(state.syncState)
                Text(
                    text = syncStateDisplayName(state.syncState),
                    style = MaterialTheme.typography.bodyMedium,
                    color = syncStateColor(state.syncState)
                )
            }

            // Show last sync result
            state.lastSyncResult?.let { result ->
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Last sync: ${result.fetchedCount} fetched, ${result.failedCount} failed",
                    style = MaterialTheme.typography.bodySmall
                )
            }

            // Show error if any
            state.lastError?.let { error ->
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Error: $error",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }
        }
    }
}

@Composable
fun PostItemCard(item: PostItemDisplayData) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(
            modifier = Modifier.padding(12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // State indicator
            StateIndicator(state = item.state)

            Spacer(modifier = Modifier.width(12.dp))

            // Content
            Column(modifier = Modifier.weight(1f)) {
                when {
                    item.isLoading -> {
                        Text(
                            text = "Loading post ${item.id}...",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                        )
                    }
                    item.errorMessage != null -> {
                        Text(
                            text = "Post ${item.id}",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "Error: ${item.errorMessage}",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error
                        )
                    }
                    item.title != null -> {
                        Text(
                            text = item.title,
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis
                        )
                        item.contentPreview?.let { preview ->
                            Text(
                                text = preview,
                                style = MaterialTheme.typography.bodySmall,
                                maxLines = 2,
                                overflow = TextOverflow.Ellipsis,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                            )
                        }
                        item.status?.let { status ->
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = status,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                    }
                    else -> {
                        Text(
                            text = "Post ${item.id} (${stateDisplayName(item.state)})",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                        )
                    }
                }
            }

            // ID badge
            Text(
                text = "#${item.id}",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
            )
        }
    }
}

@Composable
fun StateIndicator(state: PostItemState) {
    val color = when (state) {
        is PostItemState.Missing -> Color.Gray
        is PostItemState.Fetching -> Color.Blue
        is PostItemState.FetchingWithData -> Color.Blue
        is PostItemState.Fresh -> Color.Green
        is PostItemState.Stale -> Color.Yellow
        is PostItemState.Failed -> Color.Red
        is PostItemState.FailedWithData -> Color.Red
    }

    Box(
        modifier = Modifier
            .size(12.dp)
            .clip(CircleShape)
            .background(color)
    )
}

fun stateDisplayName(state: PostItemState): String = when (state) {
    is PostItemState.Missing -> "missing"
    is PostItemState.Fetching -> "fetching"
    is PostItemState.FetchingWithData -> "fetching"
    is PostItemState.Fresh -> "cached"
    is PostItemState.Stale -> "stale"
    is PostItemState.Failed -> "failed"
    is PostItemState.FailedWithData -> "failed"
}

@Composable
fun SyncStateIndicator(state: ListState) {
    Box(
        modifier = Modifier
            .size(12.dp)
            .clip(CircleShape)
            .background(syncStateColor(state))
    )
}

fun syncStateDisplayName(state: ListState): String = when (state) {
    ListState.IDLE -> "Idle"
    ListState.FETCHING_FIRST_PAGE -> "Fetching First Page"
    ListState.FETCHING_NEXT_PAGE -> "Fetching Next Page"
    ListState.ERROR -> "Error"
}

@Composable
fun syncStateColor(state: ListState): Color = when (state) {
    ListState.IDLE -> Color(0xFF2E7D32) // Dark green
    ListState.FETCHING_FIRST_PAGE -> Color.Blue
    ListState.FETCHING_NEXT_PAGE -> Color.Cyan
    ListState.ERROR -> Color.Red
}

@Composable
fun LoadNextPageCard(
    state: PostMetadataCollectionState,
    onLoadClick: () -> Unit
) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            if (state.isSyncing) {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(8.dp))
                Text("Syncing...")
            } else if (state.hasMorePages) {
                Button(
                    onClick = onLoadClick,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("Load Next Page")
                }
            } else if (state.currentPage != null) {
                Text(
                    text = "All pages loaded",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary
                )
            } else {
                Text(
                    text = "Click Refresh to load posts",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                )
            }
        }
    }
}
