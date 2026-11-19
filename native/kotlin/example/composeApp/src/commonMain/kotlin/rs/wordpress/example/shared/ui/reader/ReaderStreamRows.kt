package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile.ReaderStreamCardKind
import uniffi.wp_mobile.ReaderStreamCardSite
import uniffi.wp_mobile.ReaderStreamCardTag
import uniffi.wp_mobile_cache.ReaderPostRecord

private const val SUMMARY_LINES = 3

internal fun ReaderPostRecord.titleText(): String =
    title?.plainText()?.takeIf { it.isNotBlank() } ?: identity.url ?: "Untitled"

/** Where and when the post was published. */
private fun ReaderPostRecord.sourceText(): String =
    listOfNotNull(siteName?.plainText(), authorName?.plainText(), date?.readerText()).joinToString(" · ")

/** The post's counts, its reading time and the layout its card gets. */
private fun ReaderPostRecord.countsText(): String = listOfNotNull(
    likeCount?.let { "$it likes" },
    commentCount?.let { "$it comments" },
    readingTimeMinutes?.let { "$it min read" },
    cardType?.let { "card: ${it.name.lowercase()}" }
).joinToString(" · ")

/** What the reader did with the post: seen it, liked it, saved it, followed its site. */
private fun ReaderPostRecord.marksText(isSaved: Boolean): String = listOfNotNull(
    isSeen?.let { if (it) "Seen" else "Not seen" },
    "Liked".takeIf { iLike == true },
    "Saved".takeIf { isSaved },
    "Following".takeIf { isFollowing == true }
).joinToString(" · ")

/** A post in a stream. Tapping it opens the post, and its menu has what can be done with it. */
@Composable
internal fun ReaderPostRow(row: ReaderStreamRow.Post, state: ReaderStreamState, open: (ReaderDestination) -> Unit) {
    val post = row.item.post
    ListItem(
        headlineContent = { Text(post.titleText()) },
        supportingContent = {
            Column {
                Text(post.sourceText(), style = MaterialTheme.typography.labelMedium)
                post.summary?.let { Text(it, maxLines = SUMMARY_LINES, overflow = TextOverflow.Ellipsis) }
                Text(post.countsText(), style = MaterialTheme.typography.labelMedium)
                post.marksText(row.isSaved).takeIf { it.isNotEmpty() }?.let {
                    Text(it, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary)
                }
            }
        },
        trailingContent = { ReaderPostMenu(post, row.isSaved, state) },
        modifier = Modifier
            .testTag("reader_post")
            .clickable { open(ReaderDestination.Post(post.key, post.identity)) }
    )
}

/** What can be done with a post from its stream. */
@Composable
private fun ReaderPostMenu(post: ReaderPostRecord, isSaved: Boolean, state: ReaderStreamState) {
    var expanded by remember { mutableStateOf(false) }
    val liked = post.iLike == true
    val following = post.isFollowing == true
    val seen = post.isSeen

    @Composable
    fun Action(text: String, enabled: Boolean = true, action: () -> Unit) {
        DropdownMenuItem(
            text = { Text(text) },
            enabled = enabled,
            onClick = {
                expanded = false
                action()
            }
        )
    }

    Box {
        IconButton(
            onClick = { expanded = true },
            modifier = Modifier.semantics { contentDescription = "Actions for ${post.titleText()}" }
        ) {
            Icon(Icons.Default.MoreVert, contentDescription = null)
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            Action(if (liked) "Unlike" else "Like") { state.actions.setLiked(post, !liked) }
            Action(if (isSaved) "Unsave" else "Save") { state.actions.setSaved(post, !isSaved) }
            // Only a post from a stream that sends a seen state has one to change.
            Action(if (seen == true) "Mark as unseen" else "Mark as seen", enabled = seen != null) {
                state.actions.setSeen(post, seen != true)
            }
            Action(if (following) "Unfollow site" else "Follow site") {
                state.actions.setFollowed(post.followTarget(), !following)
            }
            Action("Block site", enabled = (post.identity.siteId ?: 0uL) != 0uL) { state.actions.blockSite(post) }
            Action("Block author", enabled = (post.authorId ?: 0uL) != 0uL) { state.actions.blockAuthor(post) }
        }
    }
}

/** A card that isn't a post: sites or tags the reader may want to follow. */
@Composable
internal fun ReaderCardRow(row: ReaderStreamRow.Card, state: ReaderStreamState, open: (ReaderDestination) -> Unit) {
    Column(modifier = Modifier.testTag("reader_card").padding(vertical = 8.dp)) {
        when (val card = row.card) {
            is ReaderStreamCardKind.RecommendedSites -> {
                ReaderSectionTitle("Card: recommended sites")
                card.sites.forEach { ReaderCardSiteRow(it, state, open) }
            }
            is ReaderStreamCardKind.NewSites -> {
                ReaderSectionTitle("Card: new sites")
                card.sites.forEach { ReaderCardSiteRow(it, state, open) }
            }
            is ReaderStreamCardKind.SuggestedTags -> {
                ReaderSectionTitle("Card: suggested tags")
                card.tags.forEach { ReaderCardTagRow(it, state, open) }
            }
        }
    }
}

@Composable
private fun ReaderCardSiteRow(site: ReaderStreamCardSite, state: ReaderStreamState, open: (ReaderDestination) -> Unit) {
    val name = site.name?.plainText() ?: site.url ?: "Unnamed site"
    val stream = site.siteId?.let { ReaderStream.Site(it) } ?: site.feedId?.let { ReaderStream.Feed(it) }
    ListItem(
        headlineContent = { Text(name) },
        supportingContent = {
            Text(
                listOfNotNull(
                    site.description?.plainText()?.takeIf { it.isNotBlank() },
                    site.subscribersCount?.let { "$it subscribers" }
                ).joinToString(" · "),
                maxLines = 2,
                overflow = TextOverflow.Ellipsis
            )
        },
        trailingContent = {
            TextButton(
                onClick = { state.actions.setFollowed(site.followTarget(), true) },
                modifier = Modifier.semantics { contentDescription = "Follow site $name" }
            ) { Text("Follow") }
        },
        modifier = Modifier.clickable(enabled = stream != null) {
            stream?.let { open(ReaderDestination.Stream(it, name)) }
        }
    )
}

@Composable
private fun ReaderCardTagRow(tag: ReaderStreamCardTag, state: ReaderStreamState, open: (ReaderDestination) -> Unit) {
    val title = tag.title?.plainText() ?: tag.slug
    val followed = tag.slug in state.followedTags
    val action = if (followed) "Unfollow" else "Follow"
    ListItem(
        headlineContent = { Text(title) },
        trailingContent = {
            TextButton(
                onClick = { state.actions.setTagFollowed(tag.slug, title, !followed) },
                modifier = Modifier.semantics { contentDescription = "$action tag ${tag.slug}" }
            ) { Text(action) }
        },
        modifier = Modifier.clickable {
            open(ReaderDestination.Stream(ReaderStream.Tag(tag.slug), "Tag: $title"))
        }
    )
}
