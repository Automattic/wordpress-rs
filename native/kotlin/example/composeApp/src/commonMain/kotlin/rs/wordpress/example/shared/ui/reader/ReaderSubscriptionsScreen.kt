package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ListItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
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
import uniffi.wp_api.ReaderSubscriptionEmailFrequency
import uniffi.wp_mobile.ReaderActionOutcome
import uniffi.wp_mobile.ReaderFollowTarget
import uniffi.wp_mobile.ReaderStream
import uniffi.wp_mobile_cache.DbTable
import uniffi.wp_mobile_cache.ReaderSubscriptionRecord

/** How often a site's new posts can be emailed, with the name to show for each. */
private val frequencies = listOf(
    ReaderSubscriptionEmailFrequency.Instantly to "Instantly",
    ReaderSubscriptionEmailFrequency.Daily to "Daily",
    ReaderSubscriptionEmailFrequency.Weekly to "Weekly"
)

private fun ReaderSubscriptionRecord.nameText(): String = name?.plainText() ?: url ?: feedUrl ?: "Unnamed"

/** The followed sites and feeds: what the screen shows, read from the cache, and what its controls do. */
@Stable
private class ReaderSubscriptionsState(session: ReaderSession, val requests: ReaderRequests) {
    private val service = session.service

    var subscriptions by mutableStateOf<List<ReaderSubscriptionRecord>>(emptyList())
        private set

    var filter by mutableStateOf("")

    /** The subscriptions whose name or address has the filter's text. */
    val shown: List<ReaderSubscriptionRecord>
        get() = subscriptions.filter {
            filter.isBlank() || it.nameText().contains(filter.trim(), ignoreCase = true) ||
                it.url.orEmpty().contains(filter.trim(), ignoreCase = true)
        }

    suspend fun reload() {
        subscriptions = io { service.subscriptions() }
    }

    fun refresh() = requests.run { "The account follows ${service.refreshSubscriptions()} sites and feeds." }

    fun unfollow(subscription: ReaderSubscriptionRecord) = requests.run {
        val target = ReaderFollowTarget(
            siteId = subscription.siteId,
            feedId = subscription.feedId,
            url = subscription.url ?: subscription.feedUrl,
            name = subscription.name
        )
        service.setFollowed(target, false).text()
    }

    /** Turns one of a site's delivery settings on or off. */
    fun deliver(what: String, change: suspend () -> ReaderActionOutcome) = requests.run { "$what: ${change().text()}" }

    fun setPostEmails(siteId: ULong, enabled: Boolean) =
        deliver("Emails for new posts") { service.setPostEmails(siteId, enabled) }

    fun setCommentEmails(siteId: ULong, enabled: Boolean) =
        deliver("Emails for new comments") { service.setCommentEmails(siteId, enabled) }

    fun setPostNotifications(siteId: ULong, enabled: Boolean) =
        deliver("Notifications for new posts") { service.setPostNotifications(siteId, enabled) }

    fun setFrequency(siteId: ULong, frequency: ReaderSubscriptionEmailFrequency, name: String) = requests.run {
        service.setPostEmailFrequency(siteId, frequency)
        "New posts are now emailed: $name."
    }
}

/**
 * The sites and feeds the account follows, from the cache, then refreshed. A site's delivery
 * settings are switches, which show their change straight away, as every other action does.
 */
@Composable
fun ReaderSubscriptionsScreen(
    session: ReaderSession,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    val requests = rememberReaderRequests()
    val state = remember(session) { ReaderSubscriptionsState(session, requests) }

    ReloadWhenCacheChanges(
        key = state,
        isRelevantUpdate = { it.table == DbTable.READER_SUBSCRIPTIONS },
        onError = requests::failed
    ) { state.reload() }
    OnFirstOpen(state) { state.refresh() }

    ReaderScaffold(title = "Subscriptions", requests = requests, onBackClicked = onBackClicked) {
        LazyColumn(modifier = Modifier.fillMaxSize().testTag("reader_subscriptions")) {
            item {
                ReaderControlsRow {
                    OutlinedTextField(
                        value = state.filter,
                        onValueChange = { state.filter = it },
                        label = { Text("Filter by name or address") },
                        singleLine = true,
                        modifier = Modifier.weight(1f)
                    )
                    TextButton(onClick = { state.refresh() }) { Text("Refresh") }
                }
            }
            item { ReaderNote("${state.shown.size} of ${state.subscriptions.size} subscriptions") }
            items(state.shown) { subscription ->
                ReaderSubscriptionRow(subscription, state, open)
                HorizontalDivider()
            }
        }
    }
}

/** A followed site or feed: its name and address, the way to unfollow it, and a site's delivery settings. */
@Composable
private fun ReaderSubscriptionRow(
    subscription: ReaderSubscriptionRecord,
    state: ReaderSubscriptionsState,
    open: (ReaderDestination) -> Unit
) {
    val name = subscription.nameText()
    val siteId = subscription.siteId
    val stream = siteId?.let { ReaderStream.Site(it) } ?: subscription.feedId?.let { ReaderStream.Feed(it) }
    Column {
        ListItem(
            headlineContent = { Text(name) },
            supportingContent = {
                Text(
                    listOfNotNull(
                        subscription.url ?: subscription.feedUrl,
                        siteId?.let { "site $it" },
                        subscription.feedId?.let { "feed $it" },
                        "waiting to be sent".takeIf { subscription.subscriptionId == null }
                    ).joinToString(" · ")
                )
            },
            trailingContent = {
                TextButton(
                    onClick = { state.unfollow(subscription) },
                    modifier = Modifier.semantics { contentDescription = "Unfollow $name" }
                ) { Text("Unfollow") }
            },
            modifier = Modifier.clickable(enabled = stream != null) {
                stream?.let { open(ReaderDestination.Stream(it, name)) }
            }
        )
        if (siteId != null) {
            ReaderDeliverySettings(subscription, siteId, state)
        }
    }
}

/** How a followed site's news reaches the reader: three switches, and how often posts are emailed. */
@Composable
private fun ReaderDeliverySettings(
    subscription: ReaderSubscriptionRecord,
    siteId: ULong,
    state: ReaderSubscriptionsState
) {
    val name = subscription.nameText()
    ReaderDeliverySwitch("Email me new posts", "Email posts of $name", subscription.emailPosts == true) {
        state.setPostEmails(siteId, it)
    }
    ReaderDeliverySwitch("Email me new comments", "Email comments of $name", subscription.emailComments == true) {
        state.setCommentEmails(siteId, it)
    }
    ReaderDeliverySwitch("Notify me of new posts", "Notify posts of $name", subscription.notifyPosts == true) {
        state.setPostNotifications(siteId, it)
    }
    val current = frequencies.firstOrNull { it.first == subscription.emailFrequency }?.second
    ReaderControlsRow {
        Text("Emailed: ${current?.lowercase() ?: "not set"}")
        frequencies.forEach { (frequency, label) ->
            FilterChip(
                selected = subscription.emailFrequency == frequency,
                onClick = { state.setFrequency(siteId, frequency, label) },
                label = { Text(label) },
                modifier = Modifier.semantics { contentDescription = "$label emails of $name" }
            )
        }
    }
}

@Composable
private fun ReaderDeliverySwitch(label: String, description: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    ReaderControlsRow {
        Switch(
            checked = checked,
            onCheckedChange = onChange,
            modifier = Modifier.semantics { contentDescription = description }
        )
        Text("$label: ${if (checked) "on" else "off"}")
    }
}
