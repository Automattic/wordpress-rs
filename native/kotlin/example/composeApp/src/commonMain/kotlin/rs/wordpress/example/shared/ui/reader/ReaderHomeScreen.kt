package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import uniffi.wp_api.ReaderInterest
import uniffi.wp_api.ReaderMenuItem
import uniffi.wp_api.ReaderMenuItemKind
import uniffi.wp_api.WpComSiteIdentifier
import uniffi.wp_mobile.ReaderCachedMenu
import uniffi.wp_mobile.ReaderLink
import uniffi.wp_mobile.ReaderSavedException
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile.readerLinkNotInReaderMessage
import uniffi.wp_mobile.readerStreamForMenuItem
import uniffi.wp_mobile_cache.DbTable

/** The tables the Reader home shows something from. */
private val homeTables = setOf(
    DbTable.READER_MENU_ITEMS,
    DbTable.READER_ACTIONS,
    DbTable.READER_SAVED_STATE,
    DbTable.READER_SAVED_POSTS
)


/** What the Reader home shows, read from the cache, and what its buttons do. */
@Stable
internal class ReaderHomeState(val session: ReaderSession, val requests: ReaderRequests) {
    private val service = session.service

    var menu by mutableStateOf<ReaderCachedMenu?>(null)
        private set

    /** How many things the reader turned on or off are waiting to be sent. */
    var pendingActions by mutableStateOf(0uL)
        private set

    /** How many saves and unsaves WordPress.com doesn't have. */
    var pendingSaves by mutableIntStateOf(0)
        private set

    /** The tags a new reader picks from, once asked for. They aren't cached. */
    var interests by mutableStateOf<List<ReaderInterest>>(emptyList())
        private set

    val followedTags: List<ReaderMenuItem> get() = menu?.followedTags.orEmpty()

    val followedSlugs: List<String> get() = followedTags.map { it.slug ?: it.key }

    suspend fun reload() {
        val menu = io { service.menu() }
        val actions = io { service.pendingActionCount() }
        val saves = io { service.saved().pendingCount().toInt() }
        this.menu = menu
        pendingActions = actions
        pendingSaves = saves
    }

    fun refreshMenu() = requests.run {
        service.refreshMenu()
        "Fetched the menu."
    }

    /** Sends what's waiting: what the reader turned on or off, and their saves. */
    fun sync() = requests.run {
        val waiting = service.syncActions()
        // The saved posts are synced even if that fails, and its error is shown with the rest.
        val saves = try {
            service.saved().sync()
            "Saved posts are up to date."
        } catch (e: ReaderSavedException) {
            "Saved posts: ${readerErrorText(e)}"
        }
        if (session.isSignedIn) {
            "$waiting actions still waiting to be sent. $saves"
        } else {
            "Nothing was sent: this reader's follows and saves stay on this device."
        }
    }

    /** Takes over, for the account, the saves and tags from while nobody was signed in. */
    fun adoptSignedOut() = requests.run {
        val saves = service.saved().adoptSignedOutSaves()
        val tags = service.adoptSignedOutTags()
        "Took over ${saves.imported} saves (${saves.pending} waiting, ${saves.rejected.size} refused, " +
            "${saves.waitingForRoom} over the limit) and $tags tags."
    }

    fun setTagFollowed(slug: String, title: String?, followed: Boolean) = requests.run {
        service.setTagFollowed(slug, title, followed).text()
    }

    fun loadInterests() = requests.load({ service.interests() }) { loaded ->
        interests = loaded
        requests.say("${loaded.size} tags to pick from.")
    }

    fun cleanUp() = requests.run {
        service.cleanUp()
        "Removed what nothing shows any more."
    }

    /** Why the last link opened nothing, in words. */
    var linkAnswer by mutableStateOf<String?>(null)
        private set

    /**
     * Finds what a link opens and opens it: a post, or a stream, such as a site's, a tag's or a
     * search. A link the Reader can't show sets [linkAnswer] to why, as does the search page's
     * address, which has no query. A site's, a feed's or a Reader address needs no account but for
     * the account's own streams; signed out, a post's address opens when the post is found by its
     * slug, and otherwise needs an account.
     */
    fun openLink(url: String, open: (ReaderDestination) -> Unit) {
        linkAnswer = null
        val openStream = { stream: ReaderStream -> open(ReaderDestination.Stream(stream, stream.title())) }
        requests.load({ service.resolveLink(url) }) { link ->
            when (link) {
                is ReaderLink.Post -> open(ReaderDestination.Post(link.post.key, link.post.identity))
                is ReaderLink.Site -> openStream(ReaderStream.Site(link.siteId))
                is ReaderLink.Feed -> openStream(ReaderStream.Feed(link.feedId))
                // Discover from an address names no tags: ask for the reader's, as the menu's does.
                is ReaderLink.Stream -> {
                    val stream = link.stream
                    val asksForTags = stream is ReaderStream.Discover && stream.tags.isEmpty() && !session.isSignedIn
                    openStream(if (asksForTags) ReaderStream.Discover(followedSlugs) else stream)
                }
                ReaderLink.Search -> linkAnswer = "That's the Reader's search. Type what to search for under Search."
                is ReaderLink.NotInReader -> linkAnswer = readerLinkNotInReaderMessage(link.reason)
            }
        }
    }

    /** Fetches a site's post by its slug and opens it. `site` is the site's ID or its domain. */
    fun openPostBySlug(site: String, slug: String, open: (ReaderDestination) -> Unit) {
        val identifier = site.toULongOrNull()?.let { WpComSiteIdentifier.Id(it) }
            ?: WpComSiteIdentifier.Slug(site)
        requests.load({ service.fetchPostBySlug(identifier, slug) }) { post ->
            open(ReaderDestination.Post(post.key, post.identity))
        }
    }

    /**
     * The stream a menu entry opens, or `null` for an entry of a kind this screen doesn't know.
     * Discover is given the followed tags when signed out, since no server knows them.
     */
    fun streamOf(item: ReaderMenuItem): ReaderStream? = when (val stream = readerStreamForMenuItem(item)) {
        is ReaderStream.Discover ->
            ReaderStream.Discover(if (session.isSignedIn) emptyList() else followedSlugs)
        // A tag a new reader picks from comes with its slug as its key.
        null -> if (item.kind is ReaderMenuItemKind.Tag) ReaderStream.Tag(item.key) else null
        else -> stream
    }
}

/**
 * The Reader's home: who the reader is, what's waiting to be sent, and every way into the Reader.
 * It shows the menu the cache has straight away, then fetches it.
 */
@Composable
fun ReaderHomeScreen(
    session: ReaderSession,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session) { ReaderHomeState(session, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = { it.table in homeTables },
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.refreshMenu() }

    ReaderScaffold(title = "Reader", requests = requests, onBackClicked = onBackClicked) {
        // The list waits for the cache, so that coming back to the screen finds the rows it was
        // scrolled to.
        if (state.menu == null) return@ReaderScaffold
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            item { ReaderIdentitySection(state) }
            readerStreamRows(state, open)
            readerTagRows(state, open)
            item { ReaderSearchSection(open) }
            item { ReaderOpenByIdSection(state, open) }
            readerLinkRows(state, open)
        }
    }
}
