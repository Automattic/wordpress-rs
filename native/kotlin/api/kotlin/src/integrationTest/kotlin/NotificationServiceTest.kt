package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import rs.wordpress.cache.kotlin.WordPressApiCache
import uniffi.wp_api.NotificationTab
import uniffi.wp_api.ReaderSavedSource
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.localizedDescription
import uniffi.wp_mobile.NotificationChangeOutcome
import uniffi.wp_mobile.NotificationNoteChange
import uniffi.wp_mobile.NotificationServiceException
import uniffi.wp_mobile.NotificationsFilter
import uniffi.wp_mobile.ReaderException
import uniffi.wp_mobile.notificationServiceErrorUserMessage
import uniffi.wp_mobile_cache.ReaderOwner
import java.util.Locale
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
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

    /** The notifications service for account 1, made with its Reader service as an app makes it. */
    private fun WpComApiClient.makeService(cache: WordPressApiCache) = makeNotificationService(
        makeReaderService(cache, ReaderOwner.Account(1uL), ReaderSavedSource.ANDROID)
    )

    private fun isPost(request: WpNetworkRequest, path: String) =
        request.method() == RequestMethod.POST && request.url().contains("/rest/v1.1$path?")

    /** Takes every write and answers the bell's flag, so nothing reaches a server. */
    private val writes = MockRequestExecutor(
        listOf(
            Stub({ isPost(it, "/notifications/read/all") }, json("""{"success":true,"count":1}""")),
            Stub({ isPost(it, "/notifications/read") }, json("""{"success":true}""")),
            Stub({ isPost(it, "/notifications/seen") }, json("""{"success":true}""")),
            Stub({ it.url().contains("/rest/v1.1/me?fields=has_unseen_notes") }, json("""{"has_unseen_notes":false}""")),
            Stub({ it.url().contains("fields=id%2Cnote_hash") }, hashes),
            Stub({ it.url().contains("ids=") }, whole),
        )
    )

    @Test
    fun notesAreMarkedReadUnreadAndSeen() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val service = WpComApiClient(WpAuthenticationProvider.none(), writes).makeService(cache)
        service.sync()

        assertEquals(NotificationChangeOutcome.SENT, service.markRead(listOf(10_000_000_002uL)))
        assertTrue(service.loadTabNotes(NotificationTab.UNREAD).isEmpty())
        assertEquals(NotificationChangeOutcome.SENT, service.markUnread(listOf(10_000_000_001uL)))
        assertEquals(listOf(10_000_000_001uL), service.loadTabNotes(NotificationTab.UNREAD).map { it.id })
        assertEquals(NotificationChangeOutcome.SENT, service.markAllRead(NotificationTab.ALL, null))
        assertTrue(service.loadTabNotes(NotificationTab.UNREAD).isEmpty())

        assertTrue(service.hasUnseenNotes())
        assertEquals(NotificationChangeOutcome.SENT, service.markSeen(null))
        assertFalse(service.hasUnseenNotes())
        assertFalse(service.fetchHasUnseenNotes())
        assertEquals(0uL, service.sendPendingChanges())
        assertEquals(0uL, service.pendingChangeCount())
    }

    @Test
    fun aSyncStoresTheNewestNotesAndANewHashDownloadsAgain() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val service = client.makeService(cache)

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

    @Test
    fun eachTabShowsItsOwnNotesAndANoteCanBeRemoved() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val service = client.makeService(cache)
        service.sync()

        assertEquals(listOf(10_000_000_002uL), service.loadTabNotes(NotificationTab.UNREAD).map { it.id })
        assertEquals(listOf(10_000_000_001uL), service.loadTabNotes(NotificationTab.COMMENTS).map { it.id })
        assertEquals(listOf(10_000_000_002uL), service.loadTabNotes(NotificationTab.LIKES).map { it.id })
        assertTrue(service.loadTabNotes(NotificationTab.SUBSCRIBERS).isEmpty())
        assertFalse(service.tabReachedEnd(NotificationTab.LIKES))
        assertEquals("Subscribers", NotificationTab.SUBSCRIBERS.localizedDescription(listOf(Locale.US)))

        // Removing a note sends no request.
        assertTrue(service.removeNote(10_000_000_002uL))
        assertTrue(service.loadTabNotes(NotificationTab.UNREAD).isEmpty())
    }

    @Test
    fun aNoteThatIsNotCachedCannotBeLikedOrFollowed() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val service = client.makeService(cache)

        val error = assertFailsWith<NotificationServiceException.NoteNotCached> {
            service.setPostLiked(10_000_000_009uL, true)
        }
        assertEquals(10_000_000_009uL, error.noteId)
        assertFailsWith<NotificationServiceException.NoteNotCached> {
            service.setCommentLiked(10_000_000_009uL, false)
        }
        assertFailsWith<NotificationServiceException.NoteNotCached> {
            service.setSiteFollowed(10_000_000_009uL, 7001uL, true)
        }
        assertEquals(null, notificationServiceErrorUserMessage(error, null))
        val refused = NotificationServiceException.ActionRefused(
            NotificationNoteChange.LIKE_COMMENT,
            ReaderException.MissingIds()
        )
        assertEquals("Your like was not added to this comment.", notificationServiceErrorUserMessage(refused, null))
    }
}
