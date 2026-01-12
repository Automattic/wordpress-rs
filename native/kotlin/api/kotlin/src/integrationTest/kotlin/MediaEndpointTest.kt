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
        assertEquals(title, response.title?.rendered)
        restoreTestServer()
    }

    @Test
    fun testCreateMediaRequestWithProgressReporting() = runTest {
        val progressUpdates = mutableListOf<ProgressUpdate>()
        var uploadStarted = false

        val uploadListener = object : WpRequestExecutor.UploadListener {
            override fun onProgressUpdate(uploadedBytes: Long, totalBytes: Long) {
                progressUpdates.add(ProgressUpdate(uploadedBytes, totalBytes))
            }

            override fun onUploadStarted(cancellableUpload: WpRequestExecutor.CancellableUpload) {
                uploadStarted = true
            }
        }

        val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
            username = TestCredentials.INSTANCE.adminUsername,
            password = TestCredentials.INSTANCE.adminPassword
        )
        val requestExecutor = WpRequestExecutor(
            interceptors = emptyList(),
            fileResolver = FileResolverMock(),
            uploadListener = uploadListener
        )
        val clientWithProgress = WpApiClient(
            wpOrgSiteApiRootUrl = TestCredentials.INSTANCE.apiRootUrl,
            authProvider = authProvider,
            requestExecutor = requestExecutor
        )

        val title = "Testing media upload with progress from Kotlin"
        val response = clientWithProgress.request { requestBuilder ->
            requestBuilder.media().create(
                params = MediaCreateParams(title = title, filePath = "test_media.jpg")
            )
        }.assertSuccessAndRetrieveData().data

        // Verify upload was successful
        assertEquals(title, response.title?.rendered)

        // Verify progress reporting worked
        assert(uploadStarted) { "Upload should have started" }
        assert(progressUpdates.isNotEmpty()) { "Should have received progress updates" }

        // Verify final progress shows completion
        val finalProgress = progressUpdates.last()
        assertEquals(
            finalProgress.uploadedBytes,
            finalProgress.totalBytes,
            "Final progress should show upload complete"
        )

        // Verify progress never decreases. Note: The /media endpoint only supports
        // single files, so this validates basic progress but not multi-file scenarios.
        var previousBytes = 0L
        progressUpdates.forEach { update ->
            assert(update.uploadedBytes >= previousBytes) {
                "Progress decreased from $previousBytes to ${update.uploadedBytes}"
            }
            previousBytes = update.uploadedBytes
        }

        restoreTestServer()
    }

    fun mediaApiClient(): WpApiClient {
        val testCredentials = TestCredentials.INSTANCE
        val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
        val requestExecutor = WpRequestExecutor(
            interceptors = emptyList(),
            fileResolver = FileResolverMock()
        )
        return WpApiClient(
            wpOrgSiteApiRootUrl = testCredentials.apiRootUrl,
            authProvider = authProvider,
            requestExecutor = requestExecutor
        )
    }

    data class ProgressUpdate(val uploadedBytes: Long, val totalBytes: Long)

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
