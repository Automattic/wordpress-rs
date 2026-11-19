package rs.wordpress.example.shared

import androidx.compose.ui.Modifier

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
