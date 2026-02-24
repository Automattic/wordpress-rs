package rs.wordpress.example.shared.ui.terms

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import org.jetbrains.compose.ui.tooling.preview.Preview
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.example.shared.ui.components.LoadingIndicator
import rs.wordpress.example.shared.ui.components.LoadingMoreIndicator
import uniffi.wp_api.TermEndpointType

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun TermListByTypeScreen(
    apiClient: WpApiClient,
    termEndpointType: TermEndpointType,
    title: String,
    viewModel: TermListByTypeViewModel = remember(termEndpointType) {
        TermListByTypeViewModel(apiClient, termEndpointType)
    },
    onBackClicked: () -> Unit = {}
) {
    val terms by viewModel.terms.collectAsState()
    val isLoading by viewModel.isLoading.collectAsState()
    val isLoadingMore by viewModel.isLoadingMore.collectAsState()
    val canLoadMore by viewModel.canLoadMore.collectAsState()

    val listState = rememberLazyListState()
    val shouldLoadMore by remember {
        derivedStateOf {
            val totalItems = listState.layoutInfo.totalItemsCount
            val lastVisibleItem = listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: 0
            totalItems > 0 && lastVisibleItem >= totalItems - 3
        }
    }

    LaunchedEffect(shouldLoadMore, canLoadMore, isLoadingMore) {
        if (shouldLoadMore && canLoadMore && !isLoadingMore) {
            viewModel.loadMore()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
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
                state = listState,
                modifier = Modifier.fillMaxSize().padding(paddingValues)
            ) {
                items(terms) { term ->
                    ListItem(
                        headlineContent = { Text(term.name) },
                        supportingContent = { Text("${term.count} posts") },
                        overlineContent = { Text(term.slug) }
                    )
                }
                if (isLoadingMore) {
                    item { LoadingMoreIndicator() }
                }
            }
        }
    }
}
