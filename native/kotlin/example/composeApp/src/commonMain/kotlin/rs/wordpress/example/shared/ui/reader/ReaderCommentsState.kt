package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.wp_api.ReaderLikesParams
import uniffi.wp_api.ReaderLikesResponse
import uniffi.wp_api.ReaderUserSuggestion
import uniffi.wp_mobile.ReaderCommentsInfo
import uniffi.wp_mobile.ReaderCommentsPageResult
import uniffi.wp_mobile.ReaderException
import uniffi.wp_mobile_cache.ReaderCommentRecord
import uniffi.wp_mobile_cache.UpdateHook

/** A comment, and how many comments deep it is: `0` for a comment on the post itself. */
internal data class ReaderCommentRow(val comment: ReaderCommentRecord, val depth: Int)

/** The users who liked a comment, once asked for. */
internal data class ReaderCommentLikes(val commentId: Long, val likes: ReaderLikesResponse)

/**
 * The comments, each followed by its replies. The cache gives them oldest first, each with the ID
 * of the comment it answers. A reply to a comment the cache doesn't have is shown as a comment on
 * the post.
 */
private fun threaded(comments: List<ReaderCommentRecord>): List<ReaderCommentRow> {
    val ids = comments.mapNotNull { it.commentId }.toSet()
    val replies = comments.groupBy { comment ->
        comment.parentId?.takeIf { it in ids && it != comment.commentId }
    }
    val rows = mutableListOf<ReaderCommentRow>()
    val shown = mutableSetOf<ReaderCommentRecord>()

    fun add(comment: ReaderCommentRecord, depth: Int) {
        if (!shown.add(comment)) return
        rows += ReaderCommentRow(comment, depth)
        comment.commentId?.let { id -> replies[id]?.forEach { add(it, depth + 1) } }
    }

    replies[null]?.forEach { add(it, 0) }
    // Replies that only answer each other have no way in from the post, so they go last.
    comments.forEach { add(it, 0) }
    return rows
}

/**
 * An `@` that ends the draft, at its start or after a space, and the letters typed after it: what
 * to suggest people for. An email address's `@` isn't one.
 */
private val mentionAtEnd = Regex("(?:^|\\s)@([\\p{L}\\p{N}_.-]*)$")

/** The letters typed after an `@` that ends the draft, or `null` when the draft doesn't end with one. */
internal fun mentionPrefix(draft: String): String? = mentionAtEnd.find(draft)?.groupValues?.get(1)

/** The draft with the person's login in place of what's typed after its last `@`. */
internal fun withMention(draft: String, prefix: String, suggestion: ReaderUserSuggestion): String =
    draft.dropLast(prefix.length) + suggestion.userLogin + " "

/** How many people are offered at a time. */
private const val MENTIONS_SHOWN = 5

private fun ReaderCommentsPageResult.text(what: String): String =
    "$what: $commentCount comments, ${if (hasMore) "more to load" else "no more"}."

/** A post's comments: what the screen shows, read from the cache, and what its buttons do. */
@Stable
internal class ReaderCommentsState(
    val session: ReaderSession,
    private val siteId: ULong,
    postId: Long,
    val requests: ReaderRequests
) {
    private val service = session.service
    private val collection = service.comments(siteId, postId)

    var rows by mutableStateOf<List<ReaderCommentRow>>(emptyList())
        private set

    var info by mutableStateOf<ReaderCommentsInfo?>(null)
        private set

    /** The comment being answered, or `null` when writing a comment on the post. */
    var replyTo by mutableStateOf<ReaderCommentRecord?>(null)

    var likes by mutableStateOf<ReaderCommentLikes?>(null)
        private set

    /** The site's people to suggest after an `@`, once fetched. */
    var mentionSuggestions by mutableStateOf<List<ReaderUserSuggestion>?>(null)
        private set

    /** Why the people to suggest couldn't be fetched. */
    var mentionError by mutableStateOf<String?>(null)
        private set

    private var fetchingMentions = false

    /** Fetches the site's people to suggest, the first time the draft ends with an `@`. */
    fun loadMentionsIfNeeded() {
        if (mentionSuggestions != null || mentionError != null || fetchingMentions) return
        fetchingMentions = true
        requests.launch {
            try {
                val response = io { collection.mentionSuggestions() }
                mentionSuggestions = response.suggestions
                if (response.unreadableSuggestions.isNotEmpty()) {
                    requests.say("${response.unreadableSuggestions.size} suggestions couldn't be read.")
                }
            } catch (e: ReaderException) {
                mentionError = readerErrorText(e)
            } finally {
                fetchingMentions = false
            }
        }
    }

    /**
     * The people whose login or name starts with [prefix], in any case, as both WordPress apps
     * match them. `null` until they're fetched.
     */
    fun mentionMatches(prefix: String): List<ReaderUserSuggestion>? = mentionSuggestions?.filter {
        it.userLogin.startsWith(prefix, ignoreCase = true) ||
            it.displayName?.plainText()?.startsWith(prefix, ignoreCase = true) == true
    }?.take(MENTIONS_SHOWN)

    fun isRelevantUpdate(hook: UpdateHook): Boolean = collection.isRelevantUpdate(hook)

    suspend fun reload() {
        val comments = io { collection.loadItems() }
        val info = io { collection.info() }
        rows = threaded(comments)
        this.info = info
    }

    fun refresh() = requests.run { collection.refresh().text("Refreshed") }

    fun loadMore() = requests.run { collection.loadMore().text("Loaded more") }

    fun setLiked(comment: ReaderCommentRecord, liked: Boolean) {
        val commentId = comment.commentId ?: return
        requests.run { service.setCommentLiked(siteId, commentId, liked).text() }
    }

    /**
     * Blocks the comment's author on this device: their comments, and the replies under them, are
     * hidden at once. The blocked sites screen unblocks them.
     */
    fun blockAuthor(comment: ReaderCommentRecord) {
        val authorId = comment.authorId?.takeIf { it != 0uL } ?: return
        requests.run {
            service.setUserBlocked(authorId, true)
            "Blocked ${comment.authorName?.plainText() ?: "the author"}: their comments are hidden."
        }
    }

    fun loadLikes(comment: ReaderCommentRecord) {
        val commentId = comment.commentId ?: return
        requests.load({ service.commentLikes(siteId, commentId, ReaderLikesParams()) }) {
            likes = ReaderCommentLikes(commentId, it)
        }
    }

    /**
     * Writes a comment, or a reply to [replyTo]. It shows as being sent straight away. If the
     * server doesn't take it, it stays, marked as not taken, and the error shows.
     */
    fun send(text: String) {
        val parent = replyTo?.commentId
        replyTo = null
        requests.run {
            collection.postComment(text, parent)
            "The comment was posted."
        }
    }

    fun sendAgain(localId: String) = requests.run {
        collection.retryComment(localId)
        "The comment was posted."
    }

    fun discard(localId: String) = requests.run {
        if (collection.discardComment(localId)) "Discarded the comment." else "There was no comment to discard."
    }
}
