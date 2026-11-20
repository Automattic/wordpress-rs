package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.wp_api.ReaderLikesParams
import uniffi.wp_api.readerLikesNextPageParams
import uniffi.wp_api.ReaderLikesResponse
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_api.ReaderUserProfile
import uniffi.wp_api.WpComUserIdentifier
import uniffi.wp_mobile.ReaderFollowTarget
import uniffi.wp_mobile.ReaderOpenedPageView
import uniffi.wp_mobile.ReaderOpenedSeen
import uniffi.wp_mobile.ReaderSavedStatus
import uniffi.wp_mobile_cache.DbTable
import uniffi.wp_mobile_cache.ReaderPostRecord

/** The tables a post's screen shows something from. */
internal val postScreenTables = setOf(
    DbTable.READER_POSTS,
    DbTable.READER_SAVED_STATE,
    DbTable.READER_SAVED_POSTS
)

private fun ReaderOpenedSeen.text(): String = when (this) {
    is ReaderOpenedSeen.Marked -> "Marked as seen on opening. ${outcome.text()}"
    ReaderOpenedSeen.AlreadySeen -> "Seen already when opened."
    ReaderOpenedSeen.NoSeenState -> "Not marked as seen: the post has no seen state."
    ReaderOpenedSeen.NotCached -> "Not marked as seen: the cache doesn't have the post."
    is ReaderOpenedSeen.Failed -> "Couldn't be marked as seen. ${readerErrorText(error)}"
}

private fun ReaderOpenedPageView.text(): String = when (this) {
    is ReaderOpenedPageView.Reported -> outcome.text()
    ReaderOpenedPageView.NotAsked -> "Page view not sent: the switch on the Reader home is off."
    is ReaderOpenedPageView.Failed -> "Page view couldn't be counted. ${readerErrorText(error)}"
}

/** A post's screen: the post from the cache, and what its buttons do. */
/** How many likes a page asks for. */
private const val LIKES_PAGE_SIZE = 20u

@Stable
internal class ReaderPostState(
    val session: ReaderSession,
    private val key: ReaderSavedKey,
    private val identity: ReaderPostSaveIdentity,
    val requests: ReaderRequests
) {
    private val service = session.service

    /** The post as the cache has it, with its content. `null` when the cache doesn't have it. */
    var post by mutableStateOf<ReaderPostRecord?>(null)
        private set

    var isSaved by mutableStateOf(false)
        private set

    /** What became of the view that opening the post may count. */
    var pageView by mutableStateOf<String?>(null)
        private set

    /** What became of marking the post as seen when it was opened. */
    var seenOnOpen by mutableStateOf<String?>(null)
        private set

    /** The users who liked the post, as far as they've been loaded. They aren't cached. */
    var likes by mutableStateOf<ReaderLikesResponse?>(null)
        private set

    /** What to ask the next page of likes with, or `null` when the last page is loaded. */
    private var nextLikes by mutableStateOf<ReaderLikesParams?>(null)

    val hasMoreLikes: Boolean get() = nextLikes != null

    /** The author's profile, once asked for. It isn't cached. */
    var author by mutableStateOf<ReaderUserProfile?>(null)
        private set

    /** The site's ID, for a post of a WordPress.com or Jetpack site. An RSS post has none. */
    val siteId: ULong? get() = identity.siteId?.takeIf { it != 0uL && !identity.isExternal }

    suspend fun reload() {
        val post = io { service.cachedPost(key) }
        val saved = io { service.saved().status(identity) == ReaderSavedStatus.SAVED }
        this.post = post
        isSaved = saved
    }

    /**
     * Brings the post up to date from WordPress.com, and reports that it was opened, which marks
     * it as seen. A view is only counted when the switch on the Reader home is on.
     */
    fun opened() {
        requests.run {
            service.fetchPost(identity)
            "Up to date with WordPress.com."
        }
        val choice = session.pageViewChoice()
        requests.launch {
            val opened = io { service.postOpened(key, choice) }
            seenOnOpen = opened.seen.text()
            pageView = opened.pageView.text()
        }
    }

    /** Likes or unlikes the post. A like counts a view when the switch on the Reader home is on. */
    fun setLiked(liked: Boolean) = requests.run {
        service.setPostLiked(identity, liked, session.pageViewChoice()).text()
    }

    fun setPostSaved(saved: Boolean) = requests.run {
        if (saved) service.saved().save(identity).text() else service.saved().unsave(identity).text()
    }

    fun setSeen(seen: Boolean) = requests.run { service.setPostSeen(identity, seen).text() }

    fun setFollowed(followed: Boolean) = requests.run {
        // Without the post, the cache's address for the site or feed is what's followed.
        val target = post?.followTarget() ?: ReaderFollowTarget(siteId = identity.siteId, feedId = identity.feedId)
        service.setFollowed(target, followed).text()
    }

    fun setCommentsSubscribed(subscribed: Boolean) = requests.run {
        service.setCommentsSubscribed(identity, subscribed).text()
    }

    fun setCommentNotifications(enabled: Boolean) = requests.run {
        service.setCommentNotifications(identity, enabled).text()
    }

    /** Loads the first page of the users who liked the post. */
    fun loadLikes() = loadLikes(ReaderLikesParams(number = LIKES_PAGE_SIZE), shown = null)

    /** Loads the page after the likes that are shown. */
    fun loadMoreLikes() {
        loadLikes(nextLikes ?: return, shown = likes)
    }

    private fun loadLikes(params: ReaderLikesParams, shown: ReaderLikesResponse?) {
        val siteId = siteId ?: return
        requests.load({ service.postLikes(siteId, identity.id.toLong(), params) }) { page ->
            val all = shown?.likes.orEmpty() + page.likes
            likes = page.copy(likes = all)
            nextLikes = readerLikesNextPageParams(page, LIKES_PAGE_SIZE)
            requests.say("Loaded ${all.size} of the post's likes.")
        }
    }

    fun loadAuthor() {
        val authorId = post?.authorId?.takeIf { it != 0uL } ?: return
        requests.load({ service.userProfile(WpComUserIdentifier.Id(authorId)) }) {
            author = it
            requests.say("Loaded the author's profile.")
        }
    }
}
