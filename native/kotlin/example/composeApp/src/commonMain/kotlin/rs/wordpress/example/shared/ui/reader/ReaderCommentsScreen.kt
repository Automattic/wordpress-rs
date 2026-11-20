package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import uniffi.wp_api.ReaderUserSuggestion
import uniffi.wp_mobile_cache.ListState
import uniffi.wp_mobile_cache.ReaderCommentRecord
import uniffi.wp_mobile_cache.ReaderCommentSendState

private const val MAX_INDENT_LEVELS = 6
private val indent = 20.dp

/**
 * A post's comments from the cache, each reply indented under the comment it answers, with a field
 * to write one. A comment written here shows straight away, as being sent, then as the server's.
 */
@Composable
fun ReaderCommentsScreen(
    session: ReaderSession,
    destination: ReaderDestination.Comments,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session, destination) {
        ReaderCommentsState(session, destination.siteId, destination.postId, requests)
    }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = state::isRelevantUpdate,
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.refresh() }

    ReaderScaffold(title = "Comments", requests = requests, onBackClicked = onBackClicked) {
        // The list waits for the cache, so that coming back to the screen finds the rows it was
        // scrolled to.
        if (state.info == null) return@ReaderScaffold
        ReaderCommentsStatus(state)
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_comments")) {
            item { ReaderNote("On: ${destination.title}") }
            item { ReaderCommentField(state) }
            items(state.rows) { row ->
                ReaderCommentItem(row, state)
                HorizontalDivider()
            }
            if (state.info?.hasMore == true) {
                item {
                    ReaderControlsRow {
                        Button(onClick = { state.loadMore() }) { Text("Load more comments") }
                    }
                }
            }
        }
    }
}

/**
 * Whether the comments are loading, how many there are, and the last error. The server's count only
 * has the comments it took, so the ones written here that it hasn't are counted on their own.
 */
@Composable
private fun ReaderCommentsStatus(state: ReaderCommentsState) {
    val info = state.info ?: return
    val taken = state.rows.count { it.comment.sendState == ReaderCommentSendState.SENT }
    val notTaken = state.rows.size - taken
    ReaderNote(
        text = listOfNotNull(
            "$taken of ${info.found?.toString() ?: "an unknown number of"} comments",
            "$notTaken not sent yet".takeIf { notTaken > 0 },
            "${info.hidden} hidden from blocked users".takeIf { info.hidden > 0uL },
            info.state.text(),
            info.lastRefreshedAt?.let { "refreshed ${it.readerText()}" } ?: "never refreshed"
        ).joinToString(" · "),
        tag = "reader_comments_status"
    )
    info.errorMessage?.takeIf { info.state == ListState.ERROR }?.let { ReaderErrorNote("Last error: $it") }
    ReaderControlsRow {
        TextButton(onClick = { state.refresh() }) { Text("Refresh comments") }
    }
}

/** The field a comment, or a reply to the comment picked with "Reply", is written in. */
@Composable
private fun ReaderCommentField(state: ReaderCommentsState) {
    var text by remember { mutableStateOf("") }
    val replyTo = state.replyTo
    Column {
        if (replyTo != null) {
            ReaderControlsRow {
                Text("Replying to ${replyTo.authorText()}", modifier = Modifier.weight(1f))
                TextButton(onClick = { state.replyTo = null }) { Text("Cancel reply") }
            }
        }
        ReaderControlsRow {
            OutlinedTextField(
                value = text,
                onValueChange = {
                    text = it
                    if (mentionPrefix(it) != null) state.loadMentionsIfNeeded()
                },
                label = {
                    Text(if (state.session.isSignedIn) "Write a comment" else "Write a comment (needs an account)")
                },
                modifier = Modifier.weight(1f)
            )
            Button(
                onClick = {
                    state.send(text)
                    // Signed out, the comment is refused and nothing keeps it, so the field does.
                    if (state.session.isSignedIn) text = ""
                },
                enabled = text.isNotBlank()
            ) { Text("Send comment") }
        }
        mentionPrefix(text)?.let { prefix ->
            ReaderMentions(state, prefix) { suggestion -> text = withMention(text, prefix, suggestion) }
        }
    }
}

/** The site's people whose login or name starts with what's typed after the `@`, to pick one. */
@Composable
private fun ReaderMentions(state: ReaderCommentsState, prefix: String, pick: (ReaderUserSuggestion) -> Unit) {
    val error = state.mentionError
    val matches = state.mentionMatches(prefix)
    Column {
        when {
            error != null -> Text("No one to suggest: $error", style = MaterialTheme.typography.bodySmall)
            matches == null -> Text("Finding people to suggest…", style = MaterialTheme.typography.bodySmall)
            matches.isEmpty() -> Text("No one on the site matches.", style = MaterialTheme.typography.bodySmall)
            else -> matches.forEach { suggestion ->
                TextButton(
                    onClick = { pick(suggestion) },
                    modifier = Modifier.testTag("mention-${suggestion.userLogin}")
                ) { Text("@${suggestion.userLogin} · ${suggestion.displayName?.plainText().orEmpty()}") }
            }
        }
    }
}

/** Who wrote the comment. One written here that the server hasn't taken has no author yet: it's the reader's. */
private fun ReaderCommentRecord.authorText(): String =
    authorName?.plainText() ?: if (localId == null) "someone" else "You"

/** A comment, indented by how deep in the thread it is, with what can be done with it. */
@Composable
private fun ReaderCommentItem(row: ReaderCommentRow, state: ReaderCommentsState) {
    val comment = row.comment
    val start = indent * row.depth.coerceAtMost(MAX_INDENT_LEVELS)
    Column(modifier = Modifier.fillMaxWidth().padding(start = 16.dp + start, end = 16.dp, top = 8.dp)) {
        Text(
            listOfNotNull(comment.authorText(), comment.date?.readerText(), "reply".takeIf { row.depth > 0 })
                .joinToString(" · "),
            style = MaterialTheme.typography.labelMedium
        )
        Text(comment.content?.htmlAsText().orEmpty())
        when (comment.sendState) {
            ReaderCommentSendState.SENT -> ReaderCommentActions(comment, state)
            ReaderCommentSendState.SENDING -> Text("Sending…", color = MaterialTheme.colorScheme.primary)
            ReaderCommentSendState.FAILED -> ReaderUnsentComment(comment, state)
        }
        state.likes?.takeIf { it.commentId == comment.commentId }?.let { liked ->
            Text(
                "Liked by: " + liked.likes.likes.joinToString { it.name ?: it.login ?: "User ${it.id}" }
                    .ifEmpty { "nobody" },
                style = MaterialTheme.typography.labelMedium
            )
        }
    }
}

/** A comment's likes, with the buttons to like it, to answer it, and to see who liked it. */
@Composable
private fun ReaderCommentActions(comment: ReaderCommentRecord, state: ReaderCommentsState) {
    val liked = comment.iLike == true
    val like = if (liked) "Unlike" else "Like"
    val author = comment.authorText()
    Row {
        TextButton(
            onClick = { state.setLiked(comment, !liked) },
            modifier = Modifier.semantics { contentDescription = "$like comment by $author" }
        ) { Text("$like (${comment.likeCount ?: 0uL})") }
        TextButton(
            onClick = { state.replyTo = comment },
            modifier = Modifier.semantics { contentDescription = "Reply to $author" }
        ) { Text("Reply") }
        TextButton(
            onClick = { state.loadLikes(comment) },
            modifier = Modifier.semantics { contentDescription = "Who liked the comment by $author" }
        ) { Text("Who liked it") }
    }
    // A guest's or a pingback's author has no ID to block by.
    if (comment.authorId?.let { it != 0uL } == true) {
        TextButton(
            onClick = { state.blockAuthor(comment) },
            modifier = Modifier.semantics { contentDescription = "Block $author" }
        ) { Text("Block author") }
    }
}

/** A comment written here that the server didn't take: why, and the ways out. */
@Composable
private fun ReaderUnsentComment(comment: ReaderCommentRecord, state: ReaderCommentsState) {
    val localId = comment.localId ?: return
    Column {
        Text(
            "Not sent: ${comment.sendError ?: "no reason was given"}",
            color = MaterialTheme.colorScheme.error,
            style = MaterialTheme.typography.bodySmall
        )
        Row {
            TextButton(onClick = { state.sendAgain(localId) }) { Text("Send again") }
            TextButton(onClick = { state.discard(localId) }) { Text("Discard") }
        }
    }
}
