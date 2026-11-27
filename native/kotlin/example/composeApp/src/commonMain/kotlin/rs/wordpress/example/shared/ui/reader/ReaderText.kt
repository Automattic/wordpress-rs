package rs.wordpress.example.shared.ui.reader

import uniffi.wp_api.ReaderSearchSort
import uniffi.wp_api.WpApiException
import uniffi.wp_api.decodeHtmlEntities
import uniffi.wp_api.localizedDescription
import rs.wordpress.example.shared.readerPageViewUserAgent
import uniffi.wp_mobile.ReaderActionOutcome
import uniffi.wp_mobile.ReaderException
import uniffi.wp_mobile.ReaderLikePageView
import uniffi.wp_mobile.ReaderPageViewChoice
import uniffi.wp_mobile.ReaderPageViewOutcome
import uniffi.wp_mobile.ReaderPostLikeResult
import uniffi.wp_mobile.ReaderPostLikedOutcome
import uniffi.wp_mobile.ReaderSavedException
import uniffi.wp_mobile.ReaderSavedOutcome
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile.readerErrorMessage
import uniffi.wp_mobile.readerSavedErrorMessage
import uniffi.wp_mobile_cache.ListState
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private val tag = Regex("<[^>]+>")
private val blockEnd = Regex("(?i)</(p|div|h[1-6]|li|blockquote|figure|tr|pre)>|<br\\s*/?>")
private val blankLines = Regex("\\n\\s*\\n\\s*")
private val wordStart = Regex("(?<=[a-z0-9])(?=[A-Z])")

/**
 * An error, in words. An API error has a message of its own, and the Reader's other errors have
 * the library's sentence for each.
 */
internal fun readerErrorText(error: Throwable): String = when (error) {
    is ReaderLikeFailedException -> error.message.orEmpty()
    is ReaderException.Api -> error.v1.localizedDescription()
    is ReaderSavedException.Api -> error.v1.localizedDescription()
    is ReaderException -> readerErrorMessage(error)
    is ReaderSavedException -> readerSavedErrorMessage(error)
    is WpApiException -> error.localizedDescription()
    else -> {
        val name = error::class.simpleName.orEmpty().replace(wordStart, " ").lowercase()
            .replaceFirstChar { it.uppercase() }
        listOf(name, error.message.orEmpty()).filter { it.isNotBlank() }.joinToString(": ")
    }
}

/** What the switch on the Reader home asks for: a view sent as the app when it's on, and none when it's off. */
internal fun ReaderSession.pageViewChoice(): ReaderPageViewChoice = if (countPageViews) {
    ReaderPageViewChoice.Count(readerPageViewUserAgent())
} else {
    ReaderPageViewChoice.DontCount
}

internal fun ReaderPageViewOutcome.text(): String = when (this) {
    ReaderPageViewOutcome.COUNTED -> "Page view counted."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_RSS_POST -> "Page view not counted: an RSS post has no site stats."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_SITE_MEMBER ->
        "Page view not counted: you're a member of this site."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_MISSING_DETAILS ->
        "Page view not counted: the cache doesn't have the post's site."
    ReaderPageViewOutcome.NOT_COUNTED_FOR_USER_AGENT ->
        "Page view not counted: the app has no User-Agent that stats count a view with."
}

internal fun ReaderLikePageView.text(): String = when (this) {
    is ReaderLikePageView.Reported -> outcome.text()
    ReaderLikePageView.NotAsked -> "Page view not sent: the switch on the Reader home is off."
    ReaderLikePageView.NotForUnlike -> "Page view not sent: an unlike counts none."
    ReaderLikePageView.NotForAlreadyLiked -> "Page view not sent: the post was liked already."
    is ReaderLikePageView.Failed -> "Page view couldn't be counted. ${readerErrorText(error)}"
}

/** A like that failed, with what became of its view, which is reported whether or not the like worked. */
internal class ReaderLikeFailedException(error: ReaderException, pageView: ReaderLikePageView) :
    Exception("${readerErrorText(error)} ${pageView.text()}", error)

/** What became of a like and of its view, in words. A like that failed is thrown, with its view's outcome. */
internal fun ReaderPostLikedOutcome.text(): String = when (val like = like) {
    is ReaderPostLikeResult.Done -> "${like.outcome.text()} ${pageView.text()}"
    is ReaderPostLikeResult.Failed -> throw ReaderLikeFailedException(like.error, pageView)
}

internal fun ReaderActionOutcome.text(): String = when (this) {
    is ReaderActionOutcome.Synced -> "Sent to WordPress.com."
    is ReaderActionOutcome.Pending -> "Waiting to be sent: $reason"
}

internal fun ReaderSavedOutcome.text(): String = when (this) {
    is ReaderSavedOutcome.Synced -> "Sent to WordPress.com."
    is ReaderSavedOutcome.Pending -> "Waiting to be sent: $reason"
}

internal fun ListState.text(): String = when (this) {
    ListState.IDLE -> "Idle"
    ListState.FETCHING_FIRST_PAGE -> "Refreshing…"
    ListState.FETCHING_NEXT_PAGE -> "Loading more…"
    ListState.ERROR -> "Failed"
}

internal fun Date.readerText(): String =
    SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.getDefault()).format(this)

/** Text the API sends as HTML, such as a title with `&amp;` in it, as plain text. */
internal fun String.plainText(): String = decodeHtmlEntities(replace(tag, "")).trim()

/** A post's or a comment's HTML as text that can be read: its tags gone, its paragraphs kept. */
internal fun String.htmlAsText(): String =
    decodeHtmlEntities(replace(blockEnd, "\n").replace(tag, ""))
        .replace(blankLines, "\n\n")
        .trim()

/** A stream's name, to show as its screen's title. */
internal fun ReaderStream.title(): String = when (this) {
    is ReaderStream.Site -> "Site $siteId"
    is ReaderStream.Feed -> "Feed $feedId"
    is ReaderStream.Tag -> "Tag: $slug"
    is ReaderStream.Search -> if (sort == ReaderSearchSort.DATE) "Newest for: $query" else "Search: $query"
    is ReaderStream.ReaderList -> "List: $owner/$slug"
    else -> fixedTitle()
}

/** The name of a stream whose title has nothing of its own in it. */
private fun ReaderStream.fixedTitle(): String = when (this) {
    is ReaderStream.Following -> "Following"
    is ReaderStream.Liked -> "Liked"
    is ReaderStream.Automattic -> "Automattic"
    is ReaderStream.P2 -> "P2"
    is ReaderStream.FreshlyPressed -> "Freshly Pressed"
    is ReaderStream.Tags -> "Latest from my tags"
    is ReaderStream.Discover -> "Discover"
    is ReaderStream.FirstPosts -> "First Posts"
    is ReaderStream.Related -> "Related posts"
    else -> "Stream"
}
