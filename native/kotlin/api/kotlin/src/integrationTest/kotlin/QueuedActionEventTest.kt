package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.take
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import rs.wordpress.cache.kotlin.WordPressApiCache
import uniffi.wp_api.ReaderPostSaveIdentity
import uniffi.wp_api.ReaderSavedSource
import uniffi.wp_api.RequestContext
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpMultipartFormRequest
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_mobile.QueuedAction
import uniffi.wp_mobile.QueuedActionState
import uniffi.wp_mobile.ReaderPageViewChoice
import uniffi.wp_mobile.queuedActionEventUserMessage
import uniffi.wp_mobile_cache.ReaderOwner
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNull

/** Hears what becomes of a like made in the Reader, as an app does. Nothing reaches a server. */
class QueuedActionEventTest {
    private val post = ReaderPostSaveIdentity(
        id = 42uL,
        siteId = 7001uL,
        feedId = null,
        feedItemId = null,
        isExternal = false,
        url = null
    )

    /** Fails every request as a device with no connection does. */
    private val offline = object : RequestExecutor {
        override suspend fun execute(request: WpNetworkRequest): WpNetworkResponse =
            throw RequestExecutionException.RequestExecutionFailed(
                statusCode = null,
                redirects = null,
                reason = RequestExecutionErrorReason.DeviceIsOfflineError(errorMessage = ""),
                requestUrl = request.url(),
                requestMethod = request.method()
            )

        override suspend fun upload(request: WpMultipartFormRequest): WpNetworkResponse =
            error("No upload is made here")

        override suspend fun sleep(millis: ULong) = Unit

        override fun cancel(context: RequestContext) = Unit
    }

    /** Refuses the like of post 42 on site 7001. */
    private val refusing = MockRequestExecutor(
        listOf(
            Stub(
                { it.method() == RequestMethod.POST && it.url().contains("/sites/7001/posts/42/likes/new") },
                WpNetworkResponse(
                    """{"error":"unauthorized","message":"You can't like this post."}""".toByteArray(),
                    403u,
                    WpNetworkHeaderMap.fromMap(mapOf("Content-Type" to "application/json")),
                    "",
                    RequestMethod.POST,
                    WpNetworkHeaderMap.empty
                )
            )
        )
    )

    private fun readerOn(cache: WordPressApiCache, executor: RequestExecutor) =
        WpComApiClient(WpAuthenticationProvider.none(), executor)
            .makeReaderService(cache, ReaderOwner.Account(1uL), ReaderSavedSource.ANDROID)

    @Test
    fun aLikeMadeOfflineIsHeardAsWaiting() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val heard = async(start = CoroutineStart.UNDISPATCHED) { cache.queuedActionEvents.first() }

        readerOn(cache, offline).setPostLiked(post, true, ReaderPageViewChoice.DontCount)

        val event = heard.await()
        assertEquals(1uL, event.userId)
        assertEquals(QueuedAction.PostLike(post), event.action)
        assertEquals(true, event.on)
        assertIs<QueuedActionState.Waiting>(event.state)
        assertNull(queuedActionEventUserMessage(event, null))
    }

    @Test
    fun aRefusedLikeIsHeardWithAMessageForTheUser() = runTest {
        val cache = WordPressApiCache()
        cache.performMigrations()
        val heard = async(start = CoroutineStart.UNDISPATCHED) { cache.queuedActionEvents.take(2).toList() }

        readerOn(cache, refusing).setPostLiked(post, true, ReaderPageViewChoice.DontCount)

        val (waiting, refused) = heard.await()
        assertIs<QueuedActionState.Waiting>(waiting.state)
        assertIs<QueuedActionState.Refused>(refused.state)
        assertEquals(QueuedAction.PostLike(post), refused.action)
        assertEquals("Your like was not added to this post.", queuedActionEventUserMessage(refused, null))
    }
}
