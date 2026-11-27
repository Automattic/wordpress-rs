package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import uniffi.wp_api.ReaderSearchSort
import uniffi.wp_mobile.ReaderStream

/** A field to follow a tag by its slug. */
@Composable
internal fun ReaderFollowTagField(state: ReaderHomeState) {
    var slug by remember { mutableStateOf("") }
    ReaderControlsRow {
        OutlinedTextField(
            value = slug,
            onValueChange = { slug = it },
            label = { Text("Tag slug to follow") },
            singleLine = true,
            modifier = Modifier.weight(1f)
        )
        Button(
            onClick = {
                state.setTagFollowed(slug.trim().lowercase(), null, true)
                slug = ""
            },
            enabled = slug.isNotBlank()
        ) { Text("Follow tag") }
    }
}

/** A search of posts, by relevance or by date. */
@Composable
internal fun ReaderSearchSection(open: (ReaderDestination) -> Unit) {
    var query by remember { mutableStateOf("") }
    var sort by remember { mutableStateOf(ReaderSearchSort.RELEVANCE) }
    Column {
        ReaderSectionTitle("Search")
        ReaderControlsRow {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("Search for") },
                singleLine = true,
                modifier = Modifier.weight(1f)
            )
            Button(
                onClick = {
                    // In the reader's language, which the service has.
                    val stream = ReaderStream.Search(query.trim(), sort)
                    open(ReaderDestination.Stream(stream, stream.title()))
                },
                enabled = query.isNotBlank()
            ) { Text("Search posts") }
        }
        ReaderControlsRow {
            FilterChip(
                selected = sort == ReaderSearchSort.RELEVANCE,
                onClick = { sort = ReaderSearchSort.RELEVANCE },
                label = { Text("Best match first") }
            )
            FilterChip(
                selected = sort == ReaderSearchSort.DATE,
                onClick = { sort = ReaderSearchSort.DATE },
                label = { Text("Newest first") }
            )
        }
    }
}

/** Opens a site's or a feed's stream by its ID, and a post by its address or its slug. */
@Composable
internal fun ReaderOpenByIdSection(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    Column {
        ReaderSectionTitle("Open by ID")
        ReaderIdField(label = "Site ID", button = "Open site") { id ->
            open(ReaderDestination.Stream(ReaderStream.Site(id), "Site $id"))
        }
        ReaderIdField(label = "Feed ID", button = "Open feed") { id ->
            open(ReaderDestination.Stream(ReaderStream.Feed(id), "Feed $id"))
        }
        ReaderSectionTitle("Open a post")
        ReaderPostAddressField(state, open)
        ReaderPostSlugFields(state, open)
    }
}

@Composable
private fun ReaderIdField(label: String, button: String, onOpen: (ULong) -> Unit) {
    var text by remember { mutableStateOf("") }
    val id = text.trim().toULongOrNull()
    ReaderControlsRow {
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            label = { Text(label) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.weight(1f)
        )
        Button(onClick = { id?.let(onOpen) }, enabled = id != null) { Text(button) }
    }
}

@Composable
private fun ReaderPostAddressField(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    var url by remember { mutableStateOf("") }
    ReaderControlsRow {
        OutlinedTextField(
            value = url,
            onValueChange = { url = it },
            label = { Text("Address of a post, site or feed") },
            singleLine = true,
            modifier = Modifier.weight(1f)
        )
        Button(onClick = { state.openLink(url.trim(), open) }, enabled = url.isNotBlank()) {
            Text("Open address")
        }
    }
    Text(
        "A site's, a feed's or a Reader address needs no account. Signed out, a post's address opens " +
            "when the Reader finds the post by its slug, and otherwise needs an account."
    )
    state.linkAnswer?.let { answer ->
        Text(answer, modifier = Modifier.testTag("reader_link_answer"))
    }
}

@Composable
private fun ReaderPostSlugFields(state: ReaderHomeState, open: (ReaderDestination) -> Unit) {
    var site by remember { mutableStateOf("") }
    var slug by remember { mutableStateOf("") }
    ReaderControlsRow {
        OutlinedTextField(
            value = site,
            onValueChange = { site = it },
            label = { Text("Site ID or domain") },
            singleLine = true,
            modifier = Modifier.weight(1f)
        )
        OutlinedTextField(
            value = slug,
            onValueChange = { slug = it },
            label = { Text("Post slug") },
            singleLine = true,
            modifier = Modifier.weight(1f)
        )
    }
    ReaderControlsRow {
        Button(
            onClick = { state.openPostBySlug(site.trim(), slug.trim(), open) },
            enabled = site.isNotBlank() && slug.isNotBlank()
        ) { Text("Open slug") }
    }
}
