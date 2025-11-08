package rs.wordpress.example.shared.ui.stresstest

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
import androidx.compose.material.Card
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import org.koin.compose.koinInject

@Composable
@Preview
fun StressTestScreen(viewModel: StressTestViewModel = koinInject()) {
    val posts by viewModel.posts.collectAsState()
    val totalUpdates by viewModel.totalUpdates.collectAsState()
    val isRunning by viewModel.isRunning.collectAsState()

    MaterialTheme {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.fillMaxSize().padding(16.dp),
        ) {
            // Metrics section
            Card(
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp),
                elevation = 4.dp
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = "Stress Test Metrics",
                        style = MaterialTheme.typography.h6,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(text = "Total Posts: ${posts.size}")
                    Text(text = "Total Updates: $totalUpdates")
                    Text(text = "Status: ${if (isRunning) "Running" else "Stopped"}")
                }
            }

            // Posts list
            LazyColumn(
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(posts) { post ->
                    PostCard(post)
                }
            }
        }
    }
}

@Composable
fun PostCard(post: PostDisplayData) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        elevation = 2.dp
    ) {
        Row(modifier = Modifier.padding(12.dp)) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = post.title,
                    style = MaterialTheme.typography.subtitle1,
                    fontWeight = FontWeight.Bold
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = post.contentPreview,
                    style = MaterialTheme.typography.body2,
                    maxLines = 2
                )
                Spacer(modifier = Modifier.height(4.dp))
                Row {
                    Text(
                        text = "Status: ${post.status}",
                        style = MaterialTheme.typography.caption
                    )
                    if (post.author != null) {
                        Text(
                            text = " • Author: ${post.author}",
                            style = MaterialTheme.typography.caption
                        )
                    }
                }
                Text(
                    text = "Modified: ${post.modified}",
                    style = MaterialTheme.typography.caption
                )
            }
            Column(
                horizontalAlignment = Alignment.End,
                modifier = Modifier.padding(start = 8.dp)
            ) {
                Text(
                    text = "Updates",
                    style = MaterialTheme.typography.caption,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "${post.updateCount}",
                    style = MaterialTheme.typography.h6,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colors.primary
                )
            }
        }
    }
}
