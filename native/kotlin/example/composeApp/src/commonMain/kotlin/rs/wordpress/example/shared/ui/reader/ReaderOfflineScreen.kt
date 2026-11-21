package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import rs.wordpress.example.shared.di.READER_OFFLINE_IMAGE_BYTES
import rs.wordpress.example.shared.di.readerOfflineImageDirectory
import rs.wordpress.example.shared.loadStoredImage
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_api.ReaderSavedOrderBy
import uniffi.wp_api.ReaderStreamOrder
import uniffi.wp_mobile.ReaderOfflinePost
import uniffi.wp_mobile.ReaderOfflineReport
import uniffi.wp_mobile.ReaderOfflineState
import uniffi.wp_mobile.ReaderSavedListItem
import uniffi.wp_mobile_cache.DbTable
import uniffi.wp_mobile_cache.UpdateHook
import java.io.File

/** How the saved list is ordered on the offline screen. */
private enum class ReaderOfflineOrder(
    val label: String,
    val orderBy: ReaderSavedOrderBy,
    val order: ReaderStreamOrder
) {
    NEWEST_SAVE("Newest save first", ReaderSavedOrderBy.DATE, ReaderStreamOrder.DESC),
    OLDEST_SAVE("Oldest save first", ReaderSavedOrderBy.DATE, ReaderStreamOrder.ASC),
    NEWEST_POST("Newest post first", ReaderSavedOrderBy.DATE_PUBLISHED, ReaderStreamOrder.DESC),
    OLDEST_POST("Oldest post first", ReaderSavedOrderBy.DATE_PUBLISHED, ReaderStreamOrder.ASC)
}

private const val BYTES_PER_MEGABYTE = 1_000_000.0

private fun ULong.megabytes(): String = "%.1f MB".format(toDouble() / BYTES_PER_MEGABYTE)

internal fun ReaderOfflineState.text(): String = when (this) {
    is ReaderOfflineState.NotDownloaded -> "Not downloaded"
    is ReaderOfflineState.Downloading -> "Downloading…"
    // A post with no images to store is whole too, so "with images" would say too much.
    is ReaderOfflineState.Ready -> if (hasImages) "Ready" else "Ready, text only"
    is ReaderOfflineState.Failed -> "Failed: $reason"
}

private fun ReaderOfflineReport.text(): String = listOfNotNull(
    "$ready ready",
    "$withImages with images",
    "$failed failed",
    "$notDownloaded not downloaded",
    stoppedBecause?.let { "stopped: $it" }
).joinToString(", ")

private fun ReaderSavedKey.text(): String = when (this) {
    is ReaderSavedKey.Post -> "post $blogId/$postId"
    is ReaderSavedKey.FeedItem -> "feed item $feedItemId"
    is ReaderSavedKey.Url -> url
}

private fun ReaderSavedListItem.titleText(): String =
    post?.title?.plainText()?.takeIf { it.isNotEmpty() } ?: post?.identity?.url ?: key.text()

/** The saved posts and what the device has of each, read from the cache, and what the buttons do. */
@Stable
private class ReaderOfflineListState(session: ReaderSession, val requests: ReaderRequests) {
    private val saved = session.service.saved()
    private var collection = saved.savedPosts()

    var order by mutableStateOf(ReaderOfflineOrder.NEWEST_SAVE)
        private set

    var items by mutableStateOf<List<ReaderSavedListItem>>(emptyList())
        private set

    var imageBytes by mutableStateOf(0uL)
        private set

    fun isRelevantUpdate(hook: UpdateHook): Boolean =
        hook.table == DbTable.READER_OFFLINE_FILES || collection.isRelevantUpdate(hook)

    suspend fun reload() {
        val collection = collection
        val items = io { collection.loadItems() }
        val imageBytes = io { saved.offlineImageBytes() }
        this.items = items
        this.imageBytes = imageBytes
    }

    fun show(order: ReaderOfflineOrder) {
        this.order = order
        collection = saved.savedPostsOrdered(order.orderBy, order.order)
        requests.launch { reload() }
    }

    /** Fetches what's missing: every saved post's content, and its images when asked. */
    fun download(includeImages: Boolean) = requests.run {
        if (includeImages) {
            saved.setOfflineImageStore(readerOfflineImageDirectory.path, READER_OFFLINE_IMAGE_BYTES)
        }
        saved.downloadForOffline(includeImages).text()
    }

    fun retry(item: ReaderSavedListItem) = requests.run {
        "${item.titleText()}: ${saved.retryOffline(item.key).text()}"
    }

    fun removeImages() = requests.run {
        // The store has to be known for its files to be deleted.
        saved.setOfflineImageStore(readerOfflineImageDirectory.path, READER_OFFLINE_IMAGE_BYTES)
        saved.removeOfflineImages()
        "The stored images were deleted."
    }

    fun loadMore() = requests.run {
        val page = collection.loadNextPage()
        "Loaded ${page.postCount} more."
    }
}

/**
 * The saved posts, with what the device has of each for reading with no connection. Nothing is
 * fetched until the reader asks.
 */
@Composable
fun ReaderOfflineScreen(
    session: ReaderSession,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session) { ReaderOfflineListState(session, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = state::isRelevantUpdate,
        onError = requests::failed
    ) { state.reload() }

    ReaderScaffold(title = "Offline reading", requests = requests, onBackClicked = onBackClicked) {
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_offline")) {
            item { ReaderOfflineControls(state) }
            item { ReaderSectionTitle("Order") }
            items(ReaderOfflineOrder.entries) { order ->
                ListItem(
                    headlineContent = { Text(order.label) },
                    trailingContent = { if (order == state.order) Text("Shown") },
                    modifier = Modifier.clickable { state.show(order) }
                )
            }
            item {
                ReaderControlsRow {
                    TextButton(onClick = { state.loadMore() }) { Text("Load more in this order") }
                }
            }
            item { ReaderSectionTitle("Saved posts (${state.items.size})") }
            if (state.items.isEmpty()) {
                item { ReaderNote("Nothing is saved.") }
            }
            items(state.items) { item -> ReaderOfflineRow(item, state, open) }
        }
    }
}

@Composable
private fun ReaderOfflineControls(state: ReaderOfflineListState) {
    ReaderControlsRow {
        Button(
            onClick = { state.download(includeImages = false) },
            modifier = Modifier.testTag("offline_download_text")
        ) { Text("Download text") }
        Button(
            onClick = { state.download(includeImages = true) },
            modifier = Modifier.testTag("offline_download_images")
        ) { Text("Text and images") }
    }
    ReaderControlsRow {
        TextButton(
            onClick = { state.removeImages() },
            modifier = Modifier.testTag("offline_remove_images")
        ) { Text("Delete stored images") }
    }
    ReaderNote(
        "Images stored: ${state.imageBytes.megabytes()} of ${READER_OFFLINE_IMAGE_BYTES.megabytes()}",
        tag = "offline_image_bytes"
    )
}

@Composable
private fun ReaderOfflineRow(
    item: ReaderSavedListItem,
    state: ReaderOfflineListState,
    open: (ReaderDestination) -> Unit
) {
    val offline = item.offline
    val isReady = offline is ReaderOfflineState.Ready
    ListItem(
        headlineContent = { Text(item.titleText()) },
        supportingContent = {
            Text(
                text = listOfNotNull(
                    offline.text(),
                    item.post?.siteName?.plainText(),
                    item.dateSaved?.let { "saved ${it.readerText()}" },
                    item.post?.date?.let { "published ${it.readerText()}" }
                ).joinToString(" · "),
                color = if (offline is ReaderOfflineState.Failed) {
                    MaterialTheme.colorScheme.error
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
                modifier = Modifier.testTag("offline_state")
            )
        },
        trailingContent = {
            if (offline is ReaderOfflineState.Failed) {
                TextButton(onClick = { state.retry(item) }) { Text("Try again") }
            }
        },
        modifier = if (isReady) {
            Modifier.testTag("offline_post").clickable { open(ReaderDestination.OfflinePost(item.key)) }
        } else {
            Modifier
        }
    )
}

/** What the offline copy screen shows: the post, and its stored images read from their files. */
private class ReaderOfflineCopy(val post: ReaderOfflinePost, val images: List<ImageBitmap>)

/**
 * A saved post as the device has it: its stored content, and its stored images read from their
 * files. It makes no request.
 */
// Whatever keeps the stored post from being read is shown, rather than lost.
@Suppress("TooGenericExceptionCaught")
@Composable
fun ReaderOfflinePostScreen(session: ReaderSession, key: ReaderSavedKey, onBackClicked: () -> Unit) {
    val requests = rememberReaderRequests()
    var copy by remember(key) { mutableStateOf<ReaderOfflineCopy?>(null) }
    var missing by remember(key) { mutableStateOf(false) }

    LaunchedEffect(session, key) {
        try {
            val post = io { session.service.saved().offlinePost(key, "") }
            if (post == null) {
                missing = true
            } else {
                val images = io {
                    post.images.mapNotNull { image ->
                        loadStoredImage(File(readerOfflineImageDirectory, image.fileName).path)
                    }
                }
                copy = ReaderOfflineCopy(post, images)
            }
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e
        } catch (e: Exception) {
            requests.failed(e)
        }
    }

    ReaderScaffold(title = "Offline copy", requests = requests, onBackClicked = onBackClicked) {
        val shown = copy
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_offline_post")) {
            if (missing) {
                item { ReaderNote("This post isn't saved, or the device doesn't have it yet.") }
            }
            if (shown != null) {
                item {
                    Text(
                        text = shown.post.post.title?.plainText() ?: key.text(),
                        style = MaterialTheme.typography.titleLarge,
                        modifier = Modifier.padding(16.dp)
                    )
                }
                item {
                    ReaderNote(
                        "${shown.post.images.size} stored images, ${shown.images.size} shown. " +
                            "Read from this device, with no request.",
                        tag = "offline_post_images"
                    )
                }
                item {
                    Text(
                        text = shown.post.post.content?.htmlAsText()
                            ?: "WordPress.com has no content for this save.",
                        modifier = Modifier.padding(16.dp).testTag("offline_post_content")
                    )
                }
                items(shown.images) { image ->
                    Image(
                        bitmap = image,
                        contentDescription = "A stored image",
                        contentScale = ContentScale.FillWidth,
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp)
                    )
                }
            }
        }
    }
}
