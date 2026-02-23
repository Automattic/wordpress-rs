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
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
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
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import org.koin.compose.koinInject
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.ui.components.PostCard
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.MockPostService
import uniffi.wp_mobile.WpService

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun StressTestScreen(
    wpService: WpService,
    account: Account.SelfHostedSite,
    onBackClicked: () -> Unit = {}
) {
    val cache = koinInject<WordPressApiCache>()
    val mockPostService = remember {
        MockPostService(cache.cache, account.domain, account.siteApiRoot)
    }
    val viewModel = remember {
        StressTestViewModel(mockPostService, wpService, cache)
    }
    DisposableEffect(viewModel) {
        onDispose { viewModel.onCleared() }
    }
    val posts by viewModel.posts.collectAsState()
    val totalUpdates by viewModel.totalUpdates.collectAsState()
    val totalInserts by viewModel.totalInserts.collectAsState()
    val totalDeletes by viewModel.totalDeletes.collectAsState()
    val isRunning by viewModel.isRunning.collectAsState()
    val performanceMetrics by viewModel.performanceMetrics.collectAsState()
    val listState = rememberLazyListState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Stress Test") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.fillMaxSize().padding(paddingValues).padding(16.dp),
        ) {
            // Metrics section
            ElevatedCard(
                modifier = Modifier.fillMaxWidth().padding(bottom = 16.dp)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        text = "Stress Test Metrics",
                        style = MaterialTheme.typography.titleLarge,
                        fontWeight = FontWeight.Bold
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Text(text = "Total Posts: ${posts.size}")
                    Text(text = "Updates: $totalUpdates | Inserts: $totalInserts | Deletes: $totalDeletes")
                    Text(text = "Total Operations: ${totalUpdates + totalInserts + totalDeletes}")
                    Text(text = "Status: ${if (isRunning) "Running" else "Stopped"}")

                    performanceMetrics?.let { metrics ->
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Performance",
                            style = MaterialTheme.typography.titleSmall,
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
