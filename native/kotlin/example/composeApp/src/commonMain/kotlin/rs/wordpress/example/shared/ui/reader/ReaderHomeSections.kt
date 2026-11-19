package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.ListItem
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import uniffi.wp_api.ReaderMenuItem
import uniffi.wp_api.ReaderMenuItemKind
import uniffi.wp_mobile.ReaderStream

/** Who the reader is, what's waiting to be sent, and the switch that counts page views. */
@Composable
internal fun ReaderIdentitySection(state: ReaderHomeState) {
    val session = state.session
    Column {
        ReaderSectionTitle("Reader")
        ReaderNote("Reading as: ${session.label}")
        ReaderNote("Waiting to be sent: ${state.pendingActions} actions, ${state.pendingSaves} saves")
        ReaderControlsRow {
            Button(onClick = { state.sync() }) { Text("Sync") }
            TextButton(onClick = { state.refreshMenu() }) { Text("Refresh menu") }
        }
        if (session.isSignedIn) {
            ReaderControlsRow {
                TextButton(onClick = { state.adoptSignedOut() }) {
                    Text("Take over signed-out saves and tags")
                }
            }
        }
        ReaderControlsRow {
            Switch(
                checked = session.countPageViews,
                onCheckedChange = { session.countPageViews = it },
                modifier = Modifier.semantics { contentDescription = "Count page views" }
            )
            Text("Count page views when a post is opened: ${if (session.countPageViews) "on" else "off"}")
        }
    }
}

/** The menu's streams and lists, with the two streams the menu doesn't have. */
internal fun LazyListScope.readerStreamRows(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    item { ReaderSectionTitle("Streams") }
    val items = state.menu?.items.orEmpty()
    if (items.isEmpty()) {
        item { ReaderNote("The menu hasn't loaded yet.") }
    }
    items(items, key = { "menu-${it.key}" }) { item ->
        val title = item.title?.plainText() ?: item.key
        val stream = state.streamOf(item)
        if (stream == null) {
            ListItem(
                headlineContent = { Text(title) },
                supportingContent = { Text("An entry of a kind this screen can't open: ${item.kind}") }
            )
        } else {
            ReaderLinkRow(title = title, subtitle = stream.title()) {
                open(ReaderDestination.Stream(stream, title))
            }
        }
    }
    item {
        ReaderLinkRow("Freshly Pressed") {
            open(ReaderDestination.Stream(ReaderStream.FreshlyPressed, "Freshly Pressed"))
        }
    }
    item {
        ReaderLinkRow("First Posts") {
            open(ReaderDestination.Stream(ReaderStream.FirstPosts, "First Posts"))
        }
    }
}

/** The followed tags, a field to follow one by its slug, and the tags to pick from. */
internal fun LazyListScope.readerTagRows(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    item { ReaderSectionTitle("Followed tags") }
    if (state.followedTags.isEmpty()) {
        item { ReaderNote("No tags are followed.") }
    } else {
        item {
            ReaderLinkRow("Latest from my tags") {
                open(ReaderDestination.Stream(ReaderStream.Tags(state.followedSlugs), "Latest from my tags"))
            }
        }
    }
    items(state.followedTags, key = { "followed-${it.key}" }) { tag ->
        ReaderTagRow(tag, followed = true, state = state, open = open)
    }
    item { ReaderFollowTagField(state) }

    item { ReaderSectionTitle("Recommended tags") }
    val followed = state.followedSlugs.toSet()
    items(state.menu?.recommendedTags.orEmpty(), key = { "recommended-${it.key}" }) { tag ->
        ReaderTagRow(tag, followed = (tag.slug ?: tag.key) in followed, state = state, open = open)
    }
    item {
        ReaderControlsRow {
            TextButton(onClick = { state.loadInterests() }) { Text("Show the tags a new reader picks from") }
        }
    }
    items(state.interests) { interest ->
        val title = interest.title?.plainText() ?: interest.slug
        ReaderTagRow(
            tag = ReaderMenuItem(key = interest.slug, kind = ReaderMenuItemKind.Tag, title = title),
            followed = interest.slug in followed,
            state = state,
            open = open,
            label = "interest"
        )
    }
}

/** A tag: tapping it opens its stream, and its button follows or unfollows it. */
@Composable
private fun ReaderTagRow(
    tag: ReaderMenuItem,
    followed: Boolean,
    state: ReaderHomeState,
    open: (ReaderDestination) -> Unit,
    label: String = "tag"
) {
    val slug = tag.slug ?: tag.key
    val title = tag.title?.plainText() ?: slug
    val action = if (followed) "Unfollow" else "Follow"
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = { Text(slug) },
        trailingContent = {
            TextButton(
                onClick = { state.setTagFollowed(slug, title, !followed) },
                modifier = Modifier.semantics { contentDescription = "$action $label $slug" }
            ) { Text(action) }
        },
        modifier = Modifier.clickable { open(ReaderDestination.Stream(ReaderStream.Tag(slug), "Tag: $title")) }
    )
}

/** The Reader's other screens, and the cache's clean-up. */
internal fun LazyListScope.readerLinkRows(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    item { ReaderSectionTitle("More") }
    item { ReaderLinkRow("Saved posts") { open(ReaderDestination.SavedPosts) } }
    item { ReaderLinkRow("Subscriptions") { open(ReaderDestination.Subscriptions) } }
    item { ReaderLinkRow("Blocked sites") { open(ReaderDestination.BlockedSites) } }
    item { ReaderLinkRow("Find sites") { open(ReaderDestination.FindSites) } }
    item {
        ReaderControlsRow {
            TextButton(onClick = { state.cleanUp() }, modifier = Modifier.padding(bottom = 24.dp)) {
                Text("Clean up the cache")
            }
        }
    }
}
