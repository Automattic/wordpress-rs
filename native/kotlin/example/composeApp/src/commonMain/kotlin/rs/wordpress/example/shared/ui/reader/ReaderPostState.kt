package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.wp_api.ReaderLikesParams
import uniffi.wp_api.ReaderLikesResponse
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_api.ReaderUserProfile
import uniffi.wp_api.WpComUserIdentifier
import uniffi.wp_mobile.ReaderFollowTarget
import uniffi.wp_mobile.ReaderPageViewOutcome
import uniffi.wp_mobile.ReaderSavedStatus
import uniffi.wp_mobile_cache.DbTable
import uniffi.wp_mobile_cache.ReaderPostRecord

/** The tables a post's screen shows something from. */
internal val postScreenTables = setOf(
    DbTable.READER_POSTS,
    DbTable.READER_SAVED_STATE,
    DbTable.READER_SAVED_POSTS
)

private fun ReaderPageViewOutcome.text(): String = when (this) {
    ReaderPageViewOutcome.COUNTED -> "Page view counted."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_RSS_POST -> "Page view not counted: an RSS post has no site stats."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_SITE_MEMBER ->
        "Page view not counted: you're a member of this site."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_MISSING_DETAILS ->
        "Page view not counted: the cache doesn't have the post's site."
}

/** A post's screen: the post from the cache, and what its buttons do. */
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

    /** The first page of the users who liked the post, once asked for. It isn't cached. */
    var likes by mutableStateOf<ReaderLikesResponse?>(null)
        private set

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
     * Brings the post up to date from WordPress.com, and counts a view of it when the switch on
     * the Reader home is on.
     */
    fun opened() {
        requests.run {
            service.fetchPost(identity)
            "Up to date with WordPress.com."
        }
        requests.launch {
            pageView = if (session.countPageViews) {
                io { service.postOpened(key) }.text()
            } else {
                "Page view not sent: the switch on the Reader home is off."
            }
        }
    }

    fun setLiked(liked: Boolean) = requests.run { service.setPostLiked(identity, liked).text() }

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
    fun loadLikes() {
        val siteId = siteId ?: return
        requests.load({ service.postLikes(siteId, identity.id.toLong(), ReaderLikesParams()) }) {
            likes = it
            requests.say("Loaded ${it.likes.size} of the post's likes.")
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
