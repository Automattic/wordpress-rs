package rs.wordpress.example.shared.ui.posttypes

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import org.koin.compose.koinInject

@Composable
@Preview
fun PostTypesScreen(
    viewModel: PostTypesViewModel = koinInject(),
    onBackClicked: (() -> Unit)? = null,
    onPostTypeClicked: (String) -> Unit = {}
) {
    val state by viewModel.state.collectAsState()
    val postTypes by viewModel.postTypes.collectAsState()

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

            // Header card with refresh button
            Card(
                modifier = Modifier.fillMaxWidth(),
                elevation = 2.dp
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text(
                            text = "Post Types",
                            style = MaterialTheme.typography.h6,
                            fontWeight = FontWeight.Bold
                        )
                        Button(
                            onClick = { viewModel.fetch() },
                            enabled = !state.isFetching
                        ) {
                            if (state.isFetching) {
                                CircularProgressIndicator(
                                    modifier = Modifier.size(16.dp),
                                    strokeWidth = 2.dp,
                                    color = MaterialTheme.colors.onPrimary
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                            }
                            Text("Refresh")
                        }
                    }
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = "${postTypes.size} post types available",
                        style = MaterialTheme.typography.body2
                    )

                    // Show error if any
                    state.lastError?.let { error ->
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Error: $error",
                            style = MaterialTheme.typography.caption,
                            color = MaterialTheme.colors.error
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            // Post types list
            if (postTypes.isEmpty() && !state.isFetching) {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    elevation = 2.dp
                ) {
                    Column(
                        modifier = Modifier.padding(32.dp),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Text(
                            text = if (state.hasFetchedOnce) {
                                "No post types found"
                            } else {
                                "Click Refresh to load post types"
                            },
                            style = MaterialTheme.typography.body1,
                            color = MaterialTheme.colors.onSurface.copy(alpha = 0.6f)
                        )
                    }
                }
            } else {
                LazyColumn(
                    modifier = Modifier.weight(1f).fillMaxSize(),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(postTypes) { postType ->
                        PostTypeCard(
                            postType = postType,
                            onClick = { onPostTypeClicked(postType.slug) }
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun PostTypeCard(
    postType: PostTypeDisplayData,
    onClick: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        elevation = 2.dp
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = postType.name,
                        style = MaterialTheme.typography.subtitle1,
                        fontWeight = FontWeight.Bold
                    )
                    Text(
                        text = postType.slug,
                        style = MaterialTheme.typography.caption,
                        color = MaterialTheme.colors.onSurface.copy(alpha = 0.6f)
                    )
                }
                Text(
                    text = "→",
                    style = MaterialTheme.typography.h6,
                    color = MaterialTheme.colors.primary
                )
            }

            postType.description?.let { description ->
                if (description.isNotBlank()) {
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(
                        text = description,
                        style = MaterialTheme.typography.body2,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                        color = MaterialTheme.colors.onSurface.copy(alpha = 0.7f)
                    )
                }
            }

            postType.restBase?.let { restBase ->
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "REST: /$restBase",
                    style = MaterialTheme.typography.overline,
                    color = MaterialTheme.colors.primary.copy(alpha = 0.7f)
                )
            }
        }
    }
}
