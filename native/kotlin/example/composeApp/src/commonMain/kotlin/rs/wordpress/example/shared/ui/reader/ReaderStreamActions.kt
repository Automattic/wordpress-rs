package rs.wordpress.example.shared.ui.reader

import uniffi.wp_mobile.ReaderFollowTarget
import uniffi.wp_mobile.ReaderService
import uniffi.wp_mobile.ReaderStreamCardSite
import uniffi.wp_mobile_cache.ReaderPostRecord
import uniffi.wp_mobile_cache.ReaderSiteRecord

/** A post's site or feed, to follow or unfollow. */
internal fun ReaderPostRecord.followTarget() = ReaderFollowTarget(
    siteId = identity.siteId,
    feedId = identity.feedId,
    url = siteUrl ?: feedUrl,
    name = siteName
)

/** A site on a card of recommended or new sites, to follow or unfollow. */
internal fun ReaderStreamCardSite.followTarget() = ReaderFollowTarget(
    siteId = siteId,
    feedId = feedId,
    url = url ?: feedUrl,
    name = name
)

/** A site or feed the cache knows, to follow or unfollow. */
internal fun ReaderSiteRecord.followTarget() = ReaderFollowTarget(
    siteId = siteId,
    feedId = feedId,
    url = url ?: feedUrl,
    name = name
)

/**
 * What a reader turns on or off from a stream. Each changes the cache before it answers, so the
 * stream shows it from the cache, not from what the call returns: that only says whether the
 * server has it, it's waiting, or it was refused, which is shown as the request's outcome.
 */
internal class ReaderStreamActions(
    private val session: ReaderSession,
    private val requests: ReaderRequests
) {
    private val service: ReaderService = session.service

    /** Likes or unlikes a post. A like counts a view when the switch on the Reader home is on. */
    fun setLiked(post: ReaderPostRecord, liked: Boolean) = requests.run {
        service.setPostLiked(post.identity, liked, session.pageViewChoice()).text()
    }

    fun setSaved(post: ReaderPostRecord, saved: Boolean) = requests.run {
        // The post is in the cache already, so its identity is all a save needs.
        if (saved) service.saved().save(post.identity).text() else service.saved().unsave(post.identity).text()
    }

    fun setSeen(post: ReaderPostRecord, seen: Boolean) = requests.run {
        service.setPostSeen(post.identity, seen).text()
    }

    fun setFollowed(target: ReaderFollowTarget, followed: Boolean) = requests.run {
        service.setFollowed(target, followed).text()
    }

    fun setTagFollowed(slug: String, title: String?, followed: Boolean) = requests.run {
        service.setTagFollowed(slug, title, followed).text()
    }

    fun blockSite(post: ReaderPostRecord) {
        val siteId = post.identity.siteId ?: return
        requests.run { "Blocked the site. " + service.setSiteBlocked(siteId, true).text() }
    }

    /** Blocks the post's author, on this device: WordPress.com keeps no list of blocked users. */
    fun blockAuthor(post: ReaderPostRecord) {
        val authorId = post.authorId ?: return
        requests.run {
            service.setUserBlocked(authorId, true)
            "Blocked the author on this device."
        }
    }
}
