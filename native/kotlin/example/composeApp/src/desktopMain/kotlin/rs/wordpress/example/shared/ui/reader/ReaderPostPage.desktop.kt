package rs.wordpress.example.shared.ui.reader

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import rs.wordpress.example.shared.ui.components.EmptyState
import uniffi.wp_mobile.ReaderLink

/** Compose for Desktop has no web view, so the desktop app can't show the post's page. */
@Composable
actual fun ReaderPostPage(
    html: String,
    postUrl: String?,
    onLink: (link: ReaderLink, url: String) -> Unit,
    onImage: (url: String) -> Unit,
    modifier: Modifier
) {
    EmptyState("The Reader's post page needs a web view. Try the Android app.", modifier)
}
