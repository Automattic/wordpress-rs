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

/** Whether the comments are loading, how many there are, and the last error. */
@Composable
private fun ReaderCommentsStatus(state: ReaderCommentsState) {
    val info = state.info ?: return
    ReaderNote(
        text = listOfNotNull(
            "${state.rows.size} of ${info.found?.toString() ?: "an unknown number of"} comments",
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
                onValueChange = { text = it },
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
    }
}

private fun ReaderCommentRecord.authorText(): String = authorName?.plainText() ?: "someone"

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
