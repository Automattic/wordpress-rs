package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.Button
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import uniffi.wp_api.ReaderFeedSearchResult
import uniffi.wp_mobile.ReaderFollowTarget
import uniffi.wp_mobile.ReaderStream

/** A site or feed a search found, and whether the reader follows it. */
private data class ReaderFoundSite(val result: ReaderFeedSearchResult, val isFollowing: Boolean) {
    /** A lookup by address sends a result without a title, so its site's or feed's name is used. */
    val name: String
        get() = (result.title ?: result.site?.name ?: result.feed?.name)?.plainText() ?: address ?: "Unnamed"

    val address: String? get() = result.url ?: result.site?.url ?: result.feed?.url ?: result.subscribeUrl

    val feedId: ULong? get() = result.feedId ?: result.site?.feedId

    val stream: ReaderStream?
        get() = result.siteId?.let { ReaderStream.Site(it) } ?: feedId?.let { ReaderStream.Feed(it) }
}

/** A search of sites and feeds. Its results aren't cached, so the screen keeps them. */
@Stable
private class ReaderFindSitesState(session: ReaderSession, val requests: ReaderRequests) {
    private val service = session.service

    var query by mutableStateOf("")

    /** What the results on screen were searched for. */
    private var searched = ""

    var sites by mutableStateOf<List<ReaderFoundSite>>(emptyList())
        private set

    var total by mutableStateOf<ULong?>(null)
        private set

    /** Where the next page starts, or `null` when there isn't one. */
    var nextOffset by mutableStateOf<UInt?>(null)
        private set

    fun search() {
        val query = query.trim()
        requests.load({ service.searchSites(query, null) }) { page ->
            searched = query
            sites = page.feeds.map { it.found() }
            total = page.total
            nextOffset = page.nextOffset
            requests.say("${page.feeds.size} results for “$query”.")
        }
    }

    fun loadMore() {
        val offset = nextOffset ?: return
        requests.load({ service.searchSites(searched, offset) }) { page ->
            sites = sites + page.feeds.map { it.found() }
            nextOffset = page.nextOffset
            requests.say("${page.feeds.size} more results.")
        }
    }

    private fun ReaderFeedSearchResult.found() =
        ReaderFoundSite(this, (site?.isFollowing ?: feed?.isFollowing) == true)

    /** Follows or unfollows a result. The search isn't cached, so the result is changed here. */
    fun setFollowed(index: Int, followed: Boolean) {
        val site = sites.getOrNull(index) ?: return
        val target = ReaderFollowTarget(
            siteId = site.result.siteId,
            feedId = site.feedId,
            url = site.result.subscribeUrl ?: site.address,
            name = site.name
        )
        requests.load({ service.setFollowed(target, followed) }) { outcome ->
            sites = sites.toMutableList().also { it[index] = site.copy(isFollowing = followed) }
            requests.say(outcome.text())
        }
    }
}

/** A search of sites and feeds, a page at a time, each result with a way to follow it. */
@Composable
fun ReaderFindSitesScreen(
    session: ReaderSession,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session) { ReaderFindSitesState(session, requests) }

    ReaderScaffold(title = "Find sites", requests = requests, onBackClicked = onBackClicked) {
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_find_sites")) {
            item {
                ReaderControlsRow {
                    OutlinedTextField(
                        value = state.query,
                        onValueChange = { state.query = it },
                        label = { Text("Site name or address") },
                        singleLine = true,
                        modifier = Modifier.weight(1f)
                    )
                    Button(onClick = { state.search() }, enabled = state.query.isNotBlank()) {
                        Text("Search sites")
                    }
                }
            }
            state.total?.let { item { ReaderNote("${state.sites.size} of $it results") } }
            itemsIndexed(state.sites) { index, site -> ReaderFoundSiteRow(index, site, state, open) }
            if (state.nextOffset != null) {
                item {
                    ReaderControlsRow {
                        Button(onClick = { state.loadMore() }) { Text("More results") }
                    }
                }
            }
        }
    }
}

@Composable
private fun ReaderFoundSiteRow(
    index: Int,
    site: ReaderFoundSite,
    state: ReaderFindSitesState,
    open: (ReaderDestination) -> Unit
) {
    val action = if (site.isFollowing) "Unfollow" else "Follow"
    val stream = site.stream
    ListItem(
        headlineContent = { Text(site.name) },
        supportingContent = {
            Text(
                listOfNotNull(
                    site.address,
                    (site.result.subscribersCount ?: site.result.feed?.subscribersCount)?.let { "$it subscribers" },
                    site.result.siteId?.let { "site $it" },
                    site.feedId?.let { "feed $it" }
                ).joinToString(" · ")
            )
        },
        trailingContent = {
            TextButton(
                onClick = { state.setFollowed(index, !site.isFollowing) },
                modifier = Modifier.semantics { contentDescription = "$action ${site.name}" }
            ) { Text(action) }
        },
        modifier = Modifier.clickable(enabled = stream != null) {
            stream?.let { open(ReaderDestination.Stream(it, site.name)) }
        }
    )
}
