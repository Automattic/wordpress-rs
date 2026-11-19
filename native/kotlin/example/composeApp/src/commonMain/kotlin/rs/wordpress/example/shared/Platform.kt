package rs.wordpress.example.shared

import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap

interface TestSiteUrl {
    val siteUrl: String
}

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform

expect fun localTestSiteUrl(): TestSiteUrl

/**
 * Lets a UI test find a `testTag` the way the platform's test tools look things up. On Android,
 * UI Automator then sees each tag as a resource ID, so the app can be driven from `adb`.
 */
expect fun Modifier.testTagsForUiTests(): Modifier

/**
 * An image stored as a file, such as one of a saved Reader post's, ready to show. `null` when
 * the platform can't read the file as an image: not every platform reads every format.
 */
expect fun loadStoredImage(path: String): ImageBitmap?

/**
 * The User-Agent the Reader's page views are sent with: what the platform's web view sends,
 * followed by the app's name, which is how the WordPress apps send a view. A site's stats read
 * the reader's browser from it, and don't count a view sent with the library's own User-Agent.
 */
expect fun readerPageViewUserAgent(): String

/** The app's name in the User-Agent of a page view. It can't have "wordpress" in it. */
internal const val READER_PAGE_VIEW_APP_NAME = "wp-rs-example/1.0"
