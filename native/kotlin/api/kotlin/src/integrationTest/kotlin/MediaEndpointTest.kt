package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.MediaCreateParams
import uniffi.wp_api.MediaListParams
import uniffi.wp_api.SparseMediaFieldWithEditContext
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

private const val MEDIA_ID_611: Long = 611

class MediaEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.parsedSiteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testMediaListRequest() = runTest {
        val mediaList = client.request { requestBuilder ->
            requestBuilder.media().listWithEditContext(params = MediaListParams())
        }.assertSuccessAndRetrieveData().data
        assert(mediaList.isNotEmpty())
    }

    @Test
    fun testRetrieveMediaRequest() = runTest {
        val media = client.request { requestBuilder ->
            requestBuilder.media().retrieveWithEditContext(MEDIA_ID_611)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(media)
    }

    @Test
    fun testFilterMediaListRequest() = runTest {
        val postList = client.request { requestBuilder ->
            requestBuilder.media().filterListWithEditContext(
                params = MediaListParams(),
                fields = listOf(
                    SparseMediaFieldWithEditContext.DATE,
                    SparseMediaFieldWithEditContext.TITLE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(postList.isNotEmpty())
        assertNull(postList.first().slug)
    }

    @Test
    fun testFilterRetrieveMediaRequest() = runTest {
        val sparseMedia = client.request { requestBuilder ->
            requestBuilder.media().filterRetrieveWithEditContext(
                mediaId = MEDIA_ID_611,
                fields = listOf(
                    SparseMediaFieldWithEditContext.DATE,
                    SparseMediaFieldWithEditContext.TITLE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNotNull(sparseMedia)
        assertNull(sparseMedia.slug)
    }

    @Test
    fun testUploadMediaRequest() = runTest {
        val title = "Testing media upload from Kotlin"
        val response = client.request { requestBuilder ->
            requestBuilder.media().upload(
                params = MediaCreateParams(title = title),
                "test_media.jpg",
                "image/jpg"
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(title, response.title.rendered)
        // TODO: Restore server
    }
}
