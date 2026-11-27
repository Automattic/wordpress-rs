package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AssistChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile_cache.ReaderPostRecord

/**
 * A post, from the cache straight away, then brought up to date from WordPress.com. Everything a
 * button changes shows from the cache, as it does in a stream.
 */
@Composable
fun ReaderPostScreen(
    session: ReaderSession,
    key: ReaderSavedKey,
    identity: ReaderPostSaveIdentity,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session, key) { ReaderPostState(session, key, identity, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = { it.table in postScreenTables },
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.opened() }

    ReaderScaffold(title = "Post", requests = requests, onBackClicked = onBackClicked) {
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_post_screen")) {
            val post = state.post
            if (post == null) {
                item { ReaderNote("The cache doesn't have this post yet.") }
            } else {
                item { ReaderPostHeading(post, state) }
                item { ReaderPostButtons(post, state, open) }
                item { ReaderPostTags(post, open) }
                item { ReaderPostLikes(state) }
                item { ReaderPostAuthor(state) }
                item { ReaderPostContent(post) }
            }
        }
    }
}

/** The post's title, where it's from, its counts, and what the reader did with it. */
@Composable
private fun ReaderPostHeading(post: ReaderPostRecord, state: ReaderPostState) {
    Column(modifier = Modifier.padding(horizontal = 16.dp)) {
        Text(post.titleText(), style = MaterialTheme.typography.headlineSmall)
        Text(
            listOfNotNull(
                post.authorName?.plainText()?.let { "By $it" },
                post.siteName?.plainText(),
                post.date?.readerText()
            ).joinToString(" · "),
            style = MaterialTheme.typography.labelLarge
        )
        Text(
            listOfNotNull(
                post.likeCount?.let { "$it likes" },
                post.commentCount?.let { "$it comments" },
                post.readingTimeMinutes?.let { "$it min read" },
                post.wordCount?.let { "$it words" }
            ).joinToString(" · "),
            style = MaterialTheme.typography.labelMedium
        )
        Text(
            listOfNotNull(
                post.isSeen?.let { if (it) "Seen" else "Not seen" },
                if (post.iLike == true) "Liked" else "Not liked",
                if (state.isSaved) "Saved" else "Not saved",
                if (post.isFollowing == true) "Following the site" else "Not following the site",
                "Subscribed to comments".takeIf { post.isSubscribedComments == true },
                "Comment notifications on".takeIf { post.receivesCommentNotifications == true }
            ).joinToString(" · "),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.primary
        )
        state.seenOnOpen?.let {
            Text(
                it,
                style = MaterialTheme.typography.labelMedium,
                modifier = Modifier.testTag("reader_post_seen_on_open")
            )
        }
        state.pageView?.let { Text(it, style = MaterialTheme.typography.labelMedium) }
        post.identity.url?.let { Text(it, style = MaterialTheme.typography.labelSmall) }
    }
}

/** What the reader can turn on or off for the post, and where the post leads. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ReaderPostButtons(post: ReaderPostRecord, state: ReaderPostState, open: (ReaderDestination) -> Unit) {
    val siteId = state.siteId
    val liked = post.iLike == true
    val following = post.isFollowing == true
    val subscribed = post.isSubscribedComments == true
    val notified = post.receivesCommentNotifications == true
    FlowRow(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        OutlinedButton(onClick = { state.setLiked(!liked) }) { Text(if (liked) "Unlike" else "Like") }
        OutlinedButton(onClick = { state.setPostSaved(!state.isSaved) }) {
            Text(if (state.isSaved) "Unsave" else "Save")
        }
        // Only a post from a stream that sends a seen state has one to change.
        OutlinedButton(onClick = { state.setSeen(post.isSeen != true) }, enabled = post.isSeen != null) {
            Text(if (post.isSeen == true) "Mark as unseen" else "Mark as seen")
        }
        OutlinedButton(onClick = { state.setFollowed(!following) }) {
            Text(if (following) "Unfollow site" else "Follow site")
        }
        OutlinedButton(onClick = { state.setCommentsSubscribed(!subscribed) }) {
            Text(if (subscribed) "Unsubscribe from comments" else "Subscribe to comments")
        }
        OutlinedButton(onClick = { state.setCommentNotifications(!notified) }, enabled = subscribed) {
            Text(if (notified) "Turn comment notifications off" else "Turn comment notifications on")
        }
        OutlinedButton(
            onClick = {
                siteId?.let { open(ReaderDestination.Comments(it, post.identity.id.toLong(), post.titleText())) }
            },
            enabled = siteId != null
        ) { Text("Comments (${post.commentCount ?: 0uL})") }
        OutlinedButton(onClick = { state.loadLikes() }, enabled = siteId != null) { Text("Who liked it") }
        OutlinedButton(
            onClick = {
                siteId?.let {
                    val related = ReaderStream.Related(it, post.identity.id.toLong())
                    open(ReaderDestination.Stream(related, "Related to: ${post.titleText()}"))
                }
            },
            enabled = siteId != null
        ) { Text("Related posts") }
        OutlinedButton(onClick = { state.loadAuthor() }, enabled = (post.authorId ?: 0uL) != 0uL) {
            Text("Author profile")
        }
    }
}

/** The post's tags. Each opens its stream. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ReaderPostTags(post: ReaderPostRecord, open: (ReaderDestination) -> Unit) {
    val tags = post.tags.orEmpty()
    if (tags.isEmpty()) return
    FlowRow(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        tags.forEach { tag ->
            val name = tag.displayName ?: tag.name
            AssistChip(
                onClick = { open(ReaderDestination.Stream(ReaderStream.Tag(tag.slug), "Tag: $name")) },
                label = { Text("#$name") }
            )
        }
    }
}

/** The users who liked the post, once asked for, and a way to load more of them. */
@Composable
private fun ReaderPostLikes(state: ReaderPostState) {
    val likes = state.likes ?: return
    Column(modifier = Modifier.testTag("reader_post_likes")) {
        ReaderSectionTitle("Who liked it")
        ReaderNote("Showing ${likes.likes.size} of ${likes.found ?: likes.likes.size.toULong()} likes.")
        likes.likes.forEach { like ->
            ReaderNote(
                listOfNotNull(like.name ?: like.login ?: "User ${like.id}", like.dateLiked?.readerText())
                    .joinToString(" · ")
            )
        }
        if (state.hasMoreLikes) {
            TextButton(
                onClick = { state.loadMoreLikes() },
                modifier = Modifier.testTag("reader_post_likes_more")
            ) { Text("Load more likes") }
        }
    }
}

@Composable
private fun ReaderPostAuthor(state: ReaderPostState) {
    val author = state.author ?: return
    Column(modifier = Modifier.testTag("reader_post_author")) {
        ReaderSectionTitle("Author")
        ReaderNote(
            listOfNotNull(
                author.displayName?.plainText(),
                author.login?.let { "@$it" },
                author.description?.plainText(),
                author.profileUrl
            ).joinToString(" · ")
        )
    }
}

/** The post's content, as text: its HTML without the tags. */
@Composable
private fun ReaderPostContent(post: ReaderPostRecord) {
    val text = remember(post.content) { post.content?.htmlAsText() }
    Column {
        ReaderSectionTitle("Content")
        Text(
            text = text?.takeIf { it.isNotBlank() } ?: "The cache has this post without its content.",
            style = MaterialTheme.typography.bodyLarge,
            modifier = Modifier.padding(start = 16.dp, end = 16.dp, bottom = 32.dp).testTag("reader_post_content")
        )
    }
}
