package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import uniffi.wp_api.MediaCreateParams
import uniffi.wp_api.MediaListParams
import uniffi.wp_api.SparseMediaFieldWithEditContext
import uniffi.wp_api.WpAuthenticationProvider
import java.io.File
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

private const val MEDIA_ID_611: Long = 611

class MediaEndpointTest {
    private val client = mediaApiClient()

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
        val mediaList = client.request { requestBuilder ->
            requestBuilder.media().filterListWithEditContext(
                params = MediaListParams(),
                fields = listOf(
                    SparseMediaFieldWithEditContext.DATE,
                    SparseMediaFieldWithEditContext.TITLE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(mediaList.isNotEmpty())
        assertNull(mediaList.first().slug)
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
    fun testCreateMediaRequest() = runTest {
        val title = "Testing media upload from Kotlin"
        val response = client.request { requestBuilder ->
            requestBuilder.media().create(
                params = MediaCreateParams(title = title, filePath = "test_media.jpg")
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(title, response.title.rendered)
        restoreTestServer()
    }

    fun mediaApiClient(): WpApiClient {
        val testCredentials = TestCredentials.INSTANCE
        val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
        val requestExecutor = WpRequestExecutor(
            fileResolver = FileResolverMock()
        )
        return WpApiClient(
            wpOrgSiteApiRootUrl = testCredentials.apiRootUrl,
            authProvider = authProvider,
            requestExecutor = requestExecutor
        )
    }

    class FileResolverMock: FileResolver {
        // in order to properly resolve the file from the test assets, we need to do it in the following way
        override fun getFile(path: String): File? =
            WpAuthenticationProvider::class.java.classLoader?.getResource(path)?.file?.let {
                File(
                    it
                )
            }
    }
}
