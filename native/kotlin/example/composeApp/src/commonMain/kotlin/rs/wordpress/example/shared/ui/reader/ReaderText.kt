package rs.wordpress.example.shared.ui.reader

import uniffi.wp_api.ReaderSearchSort
import uniffi.wp_api.WpApiException
import uniffi.wp_api.decodeHtmlEntities
import uniffi.wp_api.localizedDescription
import uniffi.wp_mobile.ReaderActionOutcome
import uniffi.wp_mobile.ReaderException
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
