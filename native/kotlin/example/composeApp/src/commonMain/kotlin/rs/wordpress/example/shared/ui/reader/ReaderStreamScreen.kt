package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile.ReaderStreamInfo
import uniffi.wp_mobile_cache.ListState
import uniffi.wp_mobile_cache.ReaderSiteRecord

/**
 * A stream's posts from the cache, with its cards in their place. It shows what the cache has
 * straight away, then refreshes, and reloads whenever the cache changes: a refresh, a page, or
 * something the reader turned on or off, here or on another screen.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReaderStreamScreen(
    session: ReaderSession,
    stream: ReaderStream,
    title: String,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session, stream) { ReaderStreamState(session, stream, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = state::isRelevantUpdate,
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.refresh() }

    ReaderScaffold(title = title, requests = requests, onBackClicked = onBackClicked) {
        // The list waits for the cache, so that coming back to the screen finds the rows it was
        // scrolled to.
        if (!state.isLoaded) return@ReaderScaffold
        ReaderStreamStatus(state)
        PullToRefreshBox(
            isRefreshing = state.isRefreshing,
            onRefresh = { state.refresh() },
            modifier = Modifier.fillMaxSize()
        ) {
            LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_stream")) {
                item { ReaderStreamHeader(state) }
                items(state.rows) { row ->
                    when (row) {
                        is ReaderStreamRow.Post -> ReaderPostRow(row, state, open)
                        is ReaderStreamRow.Gap -> ReaderGapRow(row, state)
                        is ReaderStreamRow.Card -> ReaderCardRow(row, state, open)
                    }
                    HorizontalDivider()
                }
                item { ReaderStreamFooter(state) }
            }
        }
    }
}

/** What a site's, a feed's or a tag's stream shows above its posts. */
@Composable
private fun ReaderStreamHeader(state: ReaderStreamState) {
    val stream = state.stream
    val site = state.site
    when {
        stream is ReaderStream.Tag -> ReaderTagHeader(stream.slug, state)
        site != null -> ReaderSiteHeader(site, state)
        stream is ReaderStream.Site || stream is ReaderStream.Feed ->
            ReaderNote("Nothing is cached about this ${if (stream is ReaderStream.Site) "site" else "feed"} yet.")
    }
}

@Composable
private fun ReaderSiteHeader(site: ReaderSiteRecord, state: ReaderStreamState) {
    val following = site.isFollowing == true
    Column(modifier = Modifier.testTag("reader_site_header")) {
        Text(
            text = site.name?.plainText() ?: site.url ?: "Unnamed site",
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 4.dp)
        )
        site.description?.takeIf { it.isNotBlank() }?.let { ReaderNote(it.plainText()) }
        ReaderNote(
            listOfNotNull(
                site.url,
                site.subscribersCount?.let { "$it subscribers" },
                site.postCount?.let { "$it posts" },
                if (following) "You follow it" else "You don't follow it"
            ).joinToString(" · ")
        )
        ReaderControlsRow {
            Button(onClick = { state.actions.setFollowed(site.followTarget(), !following) }) {
                Text(if (following) "Unfollow this site" else "Follow this site")
            }
        }
    }
}

@Composable
private fun ReaderTagHeader(slug: String, state: ReaderStreamState) {
    val followed = slug in state.followedTags
    val tag = state.tag
    Column {
        ReaderNote(
            listOfNotNull(
                tag?.title?.plainText() ?: slug,
                tag?.description?.plainText(),
                if (followed) "You follow it" else "You don't follow it"
            ).joinToString(" · ")
        )
        ReaderControlsRow {
            Button(onClick = { state.actions.setTagFollowed(slug, tag?.title?.plainText(), !followed) }) {
                Text(if (followed) "Unfollow this tag" else "Follow this tag")
            }
        }
    }
}

private fun ReaderStreamInfo.text(): String = listOfNotNull(
    state.text(),
    lastRefreshedAt?.let { "refreshed ${it.readerText()}" } ?: "never refreshed",
    when (hasMore) {
        true -> "more to load"
        false -> "no more to load"
        null -> null
    }
).joinToString(" · ")

/** Whether the stream is loading, its last error, when it was refreshed, and what it's missing. */
@Composable
private fun ReaderStreamStatus(state: ReaderStreamState) {
    val info = state.info ?: return
    ReaderNote(
        text = "${state.rows.count { it is ReaderStreamRow.Post }} posts · ${info.text()}",
        tag = "reader_stream_status"
    )
    info.errorMessage?.takeIf { info.state == ListState.ERROR }?.let { ReaderErrorNote("Last error: $it") }
    if (info.unreadablePostCount > 0uL) {
        ReaderControlsRow {
            Text("${info.unreadablePostCount} posts couldn't be read.", modifier = Modifier.weight(1f))
            TextButton(onClick = { state.retryUnreadablePosts() }) { Text("Try again") }
        }
    }
}

@Composable
private fun ReaderGapRow(row: ReaderStreamRow.Gap, state: ReaderStreamState) {
    ReaderControlsRow {
        Text("Posts may be missing here.", modifier = Modifier.weight(1f))
        Button(onClick = { state.fillGap(row.below) }) { Text("Load missing posts") }
    }
}

@Composable
private fun ReaderStreamFooter(state: ReaderStreamState) {
    ReaderControlsRow {
        when (state.info?.hasMore) {
            true -> Button(onClick = { state.loadMore() }, enabled = !state.requests.isRunning) { Text("Load more") }
            false -> ReaderNote("That's every post in this stream.")
            null -> ReaderNote("This stream hasn't loaded yet.")
        }
    }
}
