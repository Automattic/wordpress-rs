package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.navigation.NavController
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import rs.wordpress.example.shared.ui.components.ErrorMessage
import rs.wordpress.example.shared.ui.components.LoadingIndicator
import rs.wordpress.example.shared.ui.wpcom.WpComReaderSavedPostsScreen

/** The route every Reader screen is shown at. What each one shows is kept by [ReaderHost]. */
const val READER_ROUTE = "reader"

/**
 * Opens Reader screens on the app's navigation, and remembers what each place in the back stack
 * shows. A stream leads to a post, which leads to a tag's stream, and so on, so a screen can't
 * read "the current stream" from one shared variable: going back would show the wrong one.
 */
@Stable
class ReaderHost(private val navController: NavController, private val scope: CoroutineScope) {
    /** The reader whose Reader is open. `null` while it's being opened. */
    var session by mutableStateOf<ReaderSession?>(null)
        private set

    /** Why the reader's Reader couldn't be opened. */
    var error by mutableStateOf<String?>(null)
        private set

    /** What each back stack entry shows, by the entry's ID. */
    private val shown = mutableMapOf<String, ReaderDestination>()

    /** What was opened and has no entry yet, oldest first. */
    private val opening = ArrayDeque<ReaderDestination>()

    /**
     * Opens a reader's Reader at [destination]. [load] gets the reader's session, off the main
     * thread: opening the cache's file can take a moment.
     */
    // Whatever keeps the Reader from opening is shown, rather than lost.
    @Suppress("TooGenericExceptionCaught")
    fun start(destination: ReaderDestination, load: suspend () -> ReaderSession) {
        session = null
        error = null
        open(destination)
        scope.launch {
            try {
                session = io { load() }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = readerErrorText(e)
            }
        }
    }

    /** Opens another screen of the Reader that's open. */
    fun open(destination: ReaderDestination) {
        opening.addLast(destination)
        navController.navigate(READER_ROUTE)
    }

    /** What the back stack entry with this ID shows, or `null` if nothing was opened for it. */
    fun destinationOf(entryId: String): ReaderDestination? =
        shown[entryId] ?: opening.removeFirstOrNull()?.also { shown[entryId] = it }
}

@Composable
fun rememberReaderHost(navController: NavController): ReaderHost {
    val scope = rememberCoroutineScope()
    return remember(navController, scope) { ReaderHost(navController, scope) }
}

/**
 * A Reader screen. [session] is `null` while the reader's Reader is being opened, and [error] says
 * why it couldn't be.
 */
@Composable
fun ReaderScreen(
    session: ReaderSession?,
    error: String?,
    destination: ReaderDestination?,
    open: (ReaderDestination) -> Unit,
    onBackClicked: () -> Unit
) {
    when {
        error != null -> ErrorMessage(error)
        destination == null -> ErrorMessage("This Reader screen is gone. Go back and open it again.")
        session == null -> LoadingIndicator()
        else -> when (destination) {
            is ReaderDestination.Home -> ReaderHomeScreen(session, open, onBackClicked)
            is ReaderDestination.Stream ->
                ReaderStreamScreen(session, destination.stream, destination.title, open, onBackClicked)
            is ReaderDestination.Post ->
                ReaderPostScreen(session, destination.key, destination.identity, open, onBackClicked)
            is ReaderDestination.Comments -> ReaderCommentsScreen(session, destination, onBackClicked)
            is ReaderDestination.Subscriptions -> ReaderSubscriptionsScreen(session, open, onBackClicked)
            is ReaderDestination.BlockedSites -> ReaderBlockedSitesScreen(session, onBackClicked)
            is ReaderDestination.FindSites -> ReaderFindSitesScreen(session, open, onBackClicked)
            is ReaderDestination.SavedPosts ->
                WpComReaderSavedPostsScreen(session.service.saved(), onBackClicked)
            is ReaderDestination.Offline -> ReaderOfflineScreen(session, open, onBackClicked)
            is ReaderDestination.OfflinePost ->
                ReaderOfflinePostScreen(session, destination.key, onBackClicked)
        }
    }
}
