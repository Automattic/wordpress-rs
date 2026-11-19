package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import uniffi.wp_api.ReaderBlockedSite
import uniffi.wp_mobile_cache.DbTable

private val blockTables = setOf(DbTable.READER_BLOCKED_SITES, DbTable.READER_BLOCKED_USERS)

/** The blocked sites and users: what the screen shows, read from the cache, and what its buttons do. */
@Stable
private class ReaderBlocksState(session: ReaderSession, val requests: ReaderRequests) {
    private val service = session.service

    var sites by mutableStateOf<List<ReaderBlockedSite>>(emptyList())
        private set

    /** The users blocked on this device. WordPress.com keeps no such list. */
    var users by mutableStateOf<List<ULong>>(emptyList())
        private set

    suspend fun reload() {
        val sites = io { service.blockedSites() }
        val users = io { service.blockedUsers() }
        this.sites = sites
        this.users = users
    }

    fun refresh() = requests.run {
        service.refreshBlockedSites()
        "Fetched the blocked sites."
    }

    fun setSiteBlocked(siteId: ULong, blocked: Boolean) = requests.run {
        val what = if (blocked) "Blocked" else "Unblocked"
        "$what site $siteId. " + service.setSiteBlocked(siteId, blocked).text()
    }

    fun unblockUser(userId: ULong) = requests.run {
        service.setUserBlocked(userId, false)
        "Unblocked user $userId on this device."
    }
}

/**
 * The sites the account blocked, from the cache, then refreshed, and the users blocked on this
 * device. A blocked site's or user's posts are left out of every stream.
 */
@Composable
fun ReaderBlockedSitesScreen(session: ReaderSession, onBackClicked: () -> Unit) {
    val requests = rememberReaderRequests()
    val state = remember(session) { ReaderBlocksState(session, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = { it.table in blockTables },
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.refresh() }

    ReaderScaffold(title = "Blocked sites", requests = requests, onBackClicked = onBackClicked) {
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_blocked")) {
            item { ReaderBlockSiteField(state) }
            item { ReaderSectionTitle("Blocked sites (${state.sites.size})") }
            if (state.sites.isEmpty()) {
                item { ReaderNote("No sites are blocked.") }
            }
            items(state.sites) { site -> ReaderBlockedSiteRow(site, state) }
            item { ReaderSectionTitle("Users blocked on this device (${state.users.size})") }
            if (state.users.isEmpty()) {
                item { ReaderNote("No users are blocked.") }
            }
            items(state.users) { userId ->
                ListItem(
                    headlineContent = { Text("User $userId") },
                    trailingContent = {
                        TextButton(
                            onClick = { state.unblockUser(userId) },
                            modifier = Modifier.semantics { contentDescription = "Unblock user $userId" }
                        ) { Text("Unblock") }
                    }
                )
            }
        }
    }
}

@Composable
private fun ReaderBlockedSiteRow(site: ReaderBlockedSite, state: ReaderBlocksState) {
    ListItem(
        headlineContent = { Text(site.name?.plainText() ?: "Site ${site.id}") },
        supportingContent = { Text(listOfNotNull(site.url, "site ${site.id}").joinToString(" · ")) },
        trailingContent = {
            TextButton(
                onClick = { state.setSiteBlocked(site.id, false) },
                modifier = Modifier.semantics { contentDescription = "Unblock site ${site.id}" }
            ) { Text("Unblock") }
        }
    )
}

/** Blocks a site by its ID, for a site with no post at hand to block it from. */
@Composable
private fun ReaderBlockSiteField(state: ReaderBlocksState) {
    var text by remember { mutableStateOf("") }
    val siteId = text.trim().toULongOrNull()
    ReaderControlsRow {
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            label = { Text("Site ID to block") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.weight(1f)
        )
        Button(
            onClick = {
                siteId?.let { state.setSiteBlocked(it, true) }
                text = ""
            },
            enabled = siteId != null
        ) { Text("Block site") }
        TextButton(onClick = { state.refresh() }) { Text("Refresh") }
    }
}
