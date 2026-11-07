package rs.wordpress.example.shared.ui.wpcom

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import rs.wordpress.cache.kotlin.createObservableCollection
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_mobile.ReaderSavedException
import uniffi.wp_mobile.ReaderSavedListItem
import uniffi.wp_mobile.ReaderSavedService
import java.text.SimpleDateFormat
import java.util.Locale

private fun ReaderSavedListItem.title(): String =
    post?.title ?: post?.identity?.url ?: (key as? ReaderSavedKey.Url)?.url ?: key.toString()

private fun ReaderSavedListItem.subtitle(): String {
    val saved = when {
        isRecent -> "Saved just now"
        else -> dateSaved?.let {
            "Saved " + SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.getDefault()).format(it)
        }
    }
    val localOnly = if (isLocalOnly) "On this device only" else null
    return listOfNotNull(post?.siteName, saved, localOnly).joinToString(" · ")
}

/**
 * The saved list from the cache, through the service's collection. The rows reload whenever the
 * cache changes in a way the list shows, so a save, an unsave, a refresh and a relaunch all show up
 * without any bookkeeping in the screen.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WpComReaderSavedPostsScreen(
    readerSavedService: ReaderSavedService,
    onBackClicked: () -> Unit = {}
) {
    val collection = remember(readerSavedService) { readerSavedService.savedPosts() }
    var items by remember { mutableStateOf<List<ReaderSavedListItem>>(emptyList()) }
    var hasMore by remember { mutableStateOf(false) }
    var total by remember { mutableStateOf<ULong?>(null) }
    var urlToSave by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    val scope = rememberCoroutineScope()

    suspend fun reloadFromCache() {
        items = collection.loadItems()
        hasMore = collection.listInfo().hasMore ?: false
        total = readerSavedService.savedCount()
    }

    DisposableEffect(collection) {
        val observable = createObservableCollection(
            loadData = { collection.loadItems() },
            isRelevantUpdate = { collection.isRelevantUpdate(it) }
        )
        observable.addObserver { scope.launch { reloadFromCache() } }
        onDispose { observable.close() }
    }

    fun runRequest(request: suspend () -> Unit) {
        scope.launch {
            isLoading = true
            error = null
            try {
                request()
            } catch (e: ReaderSavedException.StaleLoadMore) {
                // A refresh started while the page loaded, and replaced it.
            } catch (e: Exception) {
                error = e.toString()
            }
            isLoading = false
        }
    }

    LaunchedEffect(collection) {
        // Show what the cache has straight away, then catch up with the server.
        reloadFromCache()
        runRequest { collection.refresh() }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(total?.let { "Saved Posts ($it)" } ?: "Saved Posts") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(paddingValues)
        ) {
            item {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(16.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    OutlinedTextField(
                        value = urlToSave,
                        onValueChange = { urlToSave = it },
                        label = { Text("URL to save") },
                        singleLine = true,
                        modifier = Modifier.weight(1f)
                    )
                    Button(
                        onClick = {
                            val url = urlToSave.trim()
                            runRequest {
                                readerSavedService.saveUrl(url, null)
                                urlToSave = ""
                            }
                        },
                        enabled = urlToSave.isNotBlank() && !isLoading
                    ) {
                        Text("Save")
                    }
                }
            }
            item {
                TextButton(
                    onClick = { runRequest { collection.refresh() } },
                    enabled = !isLoading,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text("Refresh")
                }
            }
            error?.let {
                item {
                    Text(
                        text = "Error: $it",
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.padding(16.dp)
                    )
                }
            }
            items(items, key = { it.key.toString() }) { item ->
                ListItem(
                    headlineContent = { Text(item.title()) },
                    supportingContent = { Text(item.subtitle()) },
                    trailingContent = {
                        TextButton(
                            onClick = {
                                runRequest {
                                    // The post's identity covers every key it may be saved under.
                                    val post = item.post
                                    if (post != null) {
                                        readerSavedService.unsave(post.identity)
                                    } else {
                                        readerSavedService.unsaveKey(item.key)
                                    }
                                }
                            }
                        ) {
                            Text("Unsave")
                        }
                    }
                )
            }
            if (hasMore) {
                item {
                    TextButton(
                        onClick = { runRequest { collection.loadNextPage() } },
                        enabled = !isLoading,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Load More")
                    }
                }
            }
            if (isLoading && items.isEmpty()) {
                item {
                    Box(
                        modifier = Modifier.fillMaxWidth().padding(16.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressIndicator()
                    }
                }
            }
        }
    }
}
