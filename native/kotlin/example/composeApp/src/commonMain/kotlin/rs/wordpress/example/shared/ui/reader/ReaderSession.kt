package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedKey
import uniffi.wp_mobile.ReaderService
import uniffi.wp_mobile.ReaderStream

/**
 * One reader's Reader, as the screens share it: the account's, or the reader's who isn't signed
 * in. There is one per reader for as long as the app runs, so every screen uses the same
 * [ReaderService].
 */
class ReaderSession(
    val service: ReaderService,
    /** Who the reader is, to show. */
    val label: String,
    val isSignedIn: Boolean
) {
    /**
     * Whether opening a post counts a view of it. Off until the reader turns it on, because a view
     * counted on someone's post can't be taken back.
     */
    var countPageViews by mutableStateOf(false)
}

/** A Reader screen to open, with what it shows. */
sealed interface ReaderDestination {
    data object Home : ReaderDestination

    data class Stream(val stream: ReaderStream, val title: String) : ReaderDestination

    data class Post(val key: ReaderSavedKey, val identity: ReaderPostSaveIdentity) : ReaderDestination

    data class Comments(val siteId: ULong, val postId: Long, val title: String) : ReaderDestination

    data object Subscriptions : ReaderDestination

    data object BlockedSites : ReaderDestination

    data object FindSites : ReaderDestination

    data object SavedPosts : ReaderDestination
}
