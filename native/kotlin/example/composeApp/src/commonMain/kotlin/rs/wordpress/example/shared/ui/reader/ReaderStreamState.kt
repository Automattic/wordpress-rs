package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_api.ReaderTag
import uniffi.wp_mobile.ReaderSavedStatus
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile.ReaderStreamCard
import uniffi.wp_mobile.ReaderStreamCardKind
import uniffi.wp_mobile.ReaderStreamInfo
import uniffi.wp_mobile.ReaderStreamItem
import uniffi.wp_mobile.ReaderStreamPageResult
import uniffi.wp_mobile_cache.DbTable
import uniffi.wp_mobile_cache.ReaderSiteRecord
import uniffi.wp_mobile_cache.UpdateHook

/** The tables a stream's screen shows something from, besides the stream's own. */
private val streamScreenTables = setOf(
    DbTable.READER_SAVED_STATE,
    DbTable.READER_SAVED_POSTS,
    DbTable.READER_SITES,
    DbTable.READER_MENU_ITEMS
)

/** A row of a stream's screen. */
internal sealed interface ReaderStreamRow {
    data class Post(val item: ReaderStreamItem, val isSaved: Boolean) : ReaderStreamRow

    /** Posts may be missing above the post with this key. */
    data class Gap(val below: ReaderSavedKey) : ReaderStreamRow

    data class Card(val card: ReaderStreamCardKind) : ReaderStreamRow
}

/** The stream's posts with its cards in their place, and a row for each gap. */
private fun streamRows(
    items: List<ReaderStreamItem>,
    cards: List<ReaderStreamCard>,
    saved: Set<ReaderSavedKey>
): List<ReaderStreamRow> {
    val cardsAfter = cards.groupBy { it.afterPost }
    val shown = items.map { it.post.key }.toSet()
    return buildList {
        cardsAfter[null]?.forEach { add(ReaderStreamRow.Card(it.card)) }
        for (item in items) {
            if (item.gapBefore) add(ReaderStreamRow.Gap(item.post.key))
            add(ReaderStreamRow.Post(item, item.post.key in saved))
            cardsAfter[item.post.key]?.forEach { add(ReaderStreamRow.Card(it.card)) }
        }
        // A card that comes after a post the stream leaves out, such as a blocked site's, goes last.
        cards.filter { it.afterPost != null && it.afterPost !in shown }
            .forEach { add(ReaderStreamRow.Card(it.card)) }
    }
}

private fun ReaderStreamPageResult.text(what: String): String {
    val unreadable = if (unreadablePostCount > 0uL) ", $unreadablePostCount couldn't be read" else ""
    val more = if (hasMore) "more to load" else "no more"
    return "$what: $postCount posts, $newPostCount new$unreadable, $more."
}

/** A stream's screen: what it shows, read from the cache, and what its buttons do. */
@Stable
internal class ReaderStreamState(
    val session: ReaderSession,
    val stream: ReaderStream,
    val requests: ReaderRequests
) {
    private val service = session.service
    private val collection = service.stream(stream)

    /** What the reader turns on or off from the stream's rows. */
    val actions = ReaderStreamActions(service, requests)

    var rows by mutableStateOf<List<ReaderStreamRow>>(emptyList())
        private set

    var info by mutableStateOf<ReaderStreamInfo?>(null)
        private set

    /** What the cache has of the site or feed, for a site's or a feed's stream. */
    var site by mutableStateOf<ReaderSiteRecord?>(null)
        private set

    /** The tag, for a tag's stream, once fetched. It isn't cached. */
    var tag by mutableStateOf<ReaderTag?>(null)
        private set

    var followedTags by mutableStateOf<Set<String>>(emptySet())
        private set

    var isRefreshing by mutableStateOf(false)
        private set

    /** Whether the cache has been read once. Until then there's nothing to show. */
    val isLoaded: Boolean get() = info != null

    fun isRelevantUpdate(hook: UpdateHook): Boolean =
        collection.isRelevantUpdate(hook) || hook.table in streamScreenTables

    suspend fun reload() {
        val items = io { collection.loadItems() }
        val cards = io { collection.loadCards() }
        val saved = io {
            items.filter { service.saved().status(it.post.identity) == ReaderSavedStatus.SAVED }
                .map { it.post.key }
                .toSet()
        }
        val info = io { collection.info() }
        val site = io { cachedSite() }
        val followedTags = io { service.followedTags().map { it.slug ?: it.key }.toSet() }
        rows = streamRows(items, cards, saved)
        this.info = info
        this.site = site
        this.followedTags = followedTags
    }

    private fun cachedSite(): ReaderSiteRecord? = when (stream) {
        is ReaderStream.Site -> service.cachedSite(stream.siteId, null)
        is ReaderStream.Feed -> service.cachedSite(null, stream.feedId)
        else -> null
    }

    /** Fetches the stream's first page, and what the header shows of a site, a feed or a tag. */
    fun refresh() {
        requests.launch {
            isRefreshing = true
            try {
                requests.say(io { collection.refresh() }.text("Refreshed"))
            } finally {
                isRefreshing = false
            }
        }
        when (stream) {
            is ReaderStream.Site -> requests.launch { io { service.fetchSite(stream.siteId) } }
            is ReaderStream.Feed -> requests.launch { io { service.fetchFeed(stream.feedId) } }
            is ReaderStream.Tag -> requests.load({ service.tagInfo(stream.slug) }) { tag = it }
            else -> Unit
        }
    }

    fun loadMore() = requests.run { collection.loadMore().text("Loaded more") }

    fun fillGap(below: ReaderSavedKey) = requests.run { collection.fillGap(below).text("Loaded missing posts") }

    fun retryUnreadablePosts() = requests.run {
        "${collection.retryUnreadablePosts()} of the posts that couldn't be read can be read now."
    }
}
