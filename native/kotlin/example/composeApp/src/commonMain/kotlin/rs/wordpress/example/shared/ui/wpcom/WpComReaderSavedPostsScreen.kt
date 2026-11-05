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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import org.jetbrains.compose.ui.tooling.preview.Preview
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_api.ReaderSavedPost
import uniffi.wp_api.ReaderSavedPostsParams
import uniffi.wp_api.readerSavedKeyForPost
import uniffi.wp_mobile.ReaderSavedObserver
import uniffi.wp_mobile.ReaderSavedService
import uniffi.wp_mobile.ReaderSavedStatus
import java.text.SimpleDateFormat
import java.util.Locale

private const val PAGE_SIZE = 20u

private data class SavedPostRow(
    val id: ReaderSavedKey,
    val title: String,
    val subtitle: String,
    /** The post behind this row, which a save made just now doesn't have yet. */
    val post: ReaderPostSaveIdentity?
)

private fun ReaderSavedPost.toRow(): SavedPostRow? {
    val key = readerSavedKeyForPost(identity) ?: return null
    val saved = dateSaved?.let {
        "Saved " + SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.getDefault()).format(it)
    }
    return SavedPostRow(
        id = key,
        title = title ?: identity.url ?: "Untitled",
        subtitle = listOfNotNull(siteName, saved).joinToString(" · "),
        post = identity
    )
}

/**
 * The service's recent saves above the list, without anything just unsaved. A save, an unsave or a
 * failure of either shows up here without any bookkeeping in the screen.
 */
private fun visibleRows(
    service: ReaderSavedService,
    listedPosts: List<ReaderSavedPost>,
    savedUrls: Map<ReaderSavedKey, String>
): List<SavedPostRow> {
    val listedRows = listedPosts
        .filter { service.status(it.identity) != ReaderSavedStatus.NOT_SAVED }
        .mapNotNull { it.toRow() }
    val listedKeys = listedRows.map { it.id }.toSet()
    val recentRows = service.recentSaves()
        .filter { it !in listedKeys }
        .map { SavedPostRow(it, savedUrls[it] ?: it.toString(), "Saved just now", null) }
    return recentRows + listedRows
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun WpComReaderSavedPostsScreen(
    readerSavedService: ReaderSavedService,
    onBackClicked: () -> Unit = {}
) {
    var listedPosts by remember { mutableStateOf<List<ReaderSavedPost>>(emptyList()) }
    var savedUrls by remember { mutableStateOf<Map<ReaderSavedKey, String>>(emptyMap()) }
    var nextPageHandle by remember { mutableStateOf<String?>(null) }
    var urlToSave by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    // Bumped when the service's state changes, because that state lives outside Compose.
    var revision by remember { mutableIntStateOf(0) }
    val scope = rememberCoroutineScope()

    DisposableEffect(readerSavedService) {
        readerSavedService.setObserver(
            object : ReaderSavedObserver {
                override fun savedPostsChanged() {
                    scope.launch { revision++ }
                }
            }
        )
        onDispose { readerSavedService.setObserver(null) }
    }

    suspend fun loadPage(pageHandle: String?) {
        val params = ReaderSavedPostsParams(pageHandle = pageHandle, number = PAGE_SIZE)
        val page = readerSavedService.listPosts(params)
        listedPosts = if (pageHandle == null) page.posts else listedPosts + page.posts
        nextPageHandle = page.nextPageHandle
    }

    fun runRequest(request: suspend () -> Unit) {
        scope.launch {
            isLoading = true
            error = null
            try {
                request()
            } catch (e: Exception) {
                error = e.toString()
            }
            isLoading = false
        }
    }

    LaunchedEffect(Unit) {
        runRequest {
            readerSavedService.refresh()
            loadPage(null)
        }
    }

    val rows = remember(revision, listedPosts, savedUrls) {
        visibleRows(readerSavedService, listedPosts, savedUrls)
    }
    val total = remember(revision) { readerSavedService.savedCount() }

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
                                val key = readerSavedService.saveUrl(url, null)
                                savedUrls = savedUrls + (key to url)
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
                    onClick = {
                        runRequest {
                            readerSavedService.refresh()
                            loadPage(null)
                        }
                    },
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
            items(rows, key = { it.id.toString() }) { row ->
                ListItem(
                    headlineContent = { Text(row.title) },
                    supportingContent = { Text(row.subtitle) },
                    trailingContent = {
                        TextButton(
                            onClick = {
                                runRequest {
                                    val post = row.post
                                    if (post != null) {
                                        readerSavedService.unsave(post)
                                    } else {
                                        readerSavedService.unsaveKey(row.id)
                                    }
                                }
                            }
                        ) {
                            Text("Unsave")
                        }
                    }
                )
            }
            // Only a missing handle means the end: a page can be short, or even empty, with more to come.
            nextPageHandle?.let { handle ->
                item {
                    TextButton(
                        onClick = { runRequest { loadPage(handle) } },
                        enabled = !isLoading,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Load More")
                    }
                }
            }
            if (isLoading && rows.isEmpty()) {
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
