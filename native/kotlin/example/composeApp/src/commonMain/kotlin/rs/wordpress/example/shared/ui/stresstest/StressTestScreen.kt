package rs.wordpress.example.shared.ui.stresstest

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
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
import rs.wordpress.example.shared.ui.components.PostCard

@Composable
@Preview
fun StressTestScreen(viewModel: StressTestViewModel = koinInject()) {
    val posts by viewModel.posts.collectAsState()
    val totalUpdates by viewModel.totalUpdates.collectAsState()
    val isRunning by viewModel.isRunning.collectAsState()
    val performanceMetrics by viewModel.performanceMetrics.collectAsState()
    val listState = rememberLazyListState()

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

                    performanceMetrics?.let { metrics ->
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Performance",
                            style = MaterialTheme.typography.subtitle2,
                            fontWeight = FontWeight.Bold
                        )
                        Text(text = "Avg Load: ${metrics.avgLoadTimeMs}ms (${metrics.sampleCount} samples)")
                        Text(text = "Range: ${metrics.minLoadTimeMs}ms - ${metrics.maxLoadTimeMs}ms")
                        Text(text = "Total Latency: ${metrics.avgTotalLatencyMs}ms")
                    }
                }
            }

            // Posts list
            LazyColumn(
                state = listState,
                modifier = Modifier.weight(1f).fillMaxSize(),
                verticalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                items(posts) { post ->
                    PostCard(post)
                }
            }
        }
    }
}
