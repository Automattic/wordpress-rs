package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import rs.wordpress.cache.kotlin.WordPressApiCache
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_mobile.NotificationsFilter
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** Syncs an invented account's notes through stubs, as an app does. Nothing reaches a server. */
class NotificationServiceTest {
    private fun json(body: String) = WpNetworkResponse(
        body.toByteArray(),
        200u,
        WpNetworkHeaderMap.fromMap(mapOf("Content-Type" to "application/json")),
        "",
        RequestMethod.GET,
        WpNetworkHeaderMap.empty
    )

    private val hashes = json(
        """{"last_seen_time":"1800000000","notes":[{"id":10000000002,"note_hash":21},{"id":10000000001,"note_hash":11}]}"""
    )
    private val whole = json(
        """{"last_seen_time":"1800000000","notes":[
            {"id":10000000002,"type":"like","read":0,"timestamp":"2027-01-15T08:00:02+00:00","note_hash":22},
            {"id":10000000001,"type":"comment","read":1,"timestamp":"2027-01-15T08:00:01+00:00","note_hash":12}
        ]}"""
    )

    private val executor = MockRequestExecutor(
        listOf(
            Stub({ it.url().contains("fields=id%2Cnote_hash") }, hashes),
            Stub({ it.url().contains("ids=") }, whole),
        )
    )

    private val client = WpComApiClient(WpAuthenticationProvider.none(), executor)

    @Test
    fun aSyncStoresTheNewestNotesAndANewHashDownloadsAgain() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val service = client.makeNotificationService(cache, 1uL)

        val first = service.sync()
        assertEquals(2uL, first.downloadedCount)
        assertEquals(
            listOf(10_000_000_002uL, 10_000_000_001uL),
            service.loadNotes(NotificationsFilter()).map { it.id }
        )

        // The check still gives the hashes it gave before the download, so both download again.
        assertEquals(2uL, service.sync().downloadedCount)
        assertEquals(1, service.loadNotes(NotificationsFilter(unreadOnly = true)).size)
        // Two notes on a page of 100: every note there is.
        assertTrue(service.info().reachedEnd)
    }
}
