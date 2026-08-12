package rs.wordpress.api.kotlin

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.test.runTest
import okhttp3.OkHttpClient
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import okhttp3.mockwebserver.SocketPolicy
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import uniffi.wp_api.MediaCreateParams
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import java.io.File
import java.net.URI
import java.util.concurrent.TimeUnit
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

class WpRequestExecutorTest {
    private lateinit var mockWebServer: MockWebServer

    @BeforeEach
    fun setUp() {
        mockWebServer = MockWebServer()
        mockWebServer.start()
    }

    @AfterEach
    fun tearDown() {
        mockWebServer.shutdown()
    }

    @Test
    fun `timeout exception is mapped to HttpTimeoutError`() = runTest {
        // Configure the mock server to not respond (causing a timeout)
        mockWebServer.enqueue(
            MockResponse()
                .setSocketPolicy(SocketPolicy.NO_RESPONSE)
        )

        // Create an OkHttp client with a very short timeout
        val client = OkHttpClient.Builder()
            .connectTimeout(100, TimeUnit.MILLISECONDS)
            .readTimeout(100, TimeUnit.MILLISECONDS)
            .writeTimeout(100, TimeUnit.MILLISECONDS)
            .build()

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(client),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true }
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        // Make a request that will timeout
        val result = apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }

        // Verify the error is categorized as HttpTimeoutError
        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        assertIs<RequestExecutionErrorReason.HttpTimeoutError>(
            (result as WpRequestResult.RequestExecutionFailed<*>).reason
        )
    }

    @Test
    fun `whole-call callTimeout is mapped to HttpTimeoutError, not CancellationError`() = runTest {
        // OkHttp's `callTimeout` bounds the entire call and, on expiry, cancels the call internally
        // and throws `InterruptedIOException`. Because that cancel flips `call.isCanceled()` to true,
        // the executor must classify it as a timeout *before* the `isCanceled()` cancellation check —
        // otherwise a genuine timeout would be mislabeled as a user `CancellationError`.
        mockWebServer.enqueue(
            MockResponse().setSocketPolicy(SocketPolicy.NO_RESPONSE)
        )

        // Only `callTimeout` is short; leave connect/read/write at their defaults so the whole-call
        // deadline (not a socket read timeout) is what fires.
        val client = OkHttpClient.Builder()
            .callTimeout(300, TimeUnit.MILLISECONDS)
            .build()

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(client),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true }
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        val result = apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }

        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        assertIs<RequestExecutionErrorReason.HttpTimeoutError>(
            (result as WpRequestResult.RequestExecutionFailed<*>).reason
        )
    }

    @Test
    fun `cancelling an in-flight request via CancellableCall is mapped to CancellationError`() = runTest {
        // `CancellableCall` is only surfaced through the upload path's `onUploadStarted` callback,
        // so cancellation is exercised via a media upload. Cancelling here — before `call.execute()`
        // runs — makes OkHttp throw `IOException("Canceled")`, which must be classified as
        // `CancellationError` rather than falling through to `GenericError` (matching Swift's
        // `URLError.cancelled` handling).
        val cancellingUploadListener = object : WpRequestExecutor.UploadListener {
            override fun onProgressUpdate(uploadedBytes: Long, totalBytes: Long) {
                // Not relevant to this test.
            }

            override fun onUploadStarted(cancellableUpload: WpRequestExecutor.CancellableUpload) {
                cancellableUpload.cancel()
            }
        }

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true },
            fileResolver = ClasspathFileResolver(),
            uploadListener = cancellingUploadListener
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        val result = apiClient.request { requestBuilder ->
            requestBuilder.media().create(
                params = MediaCreateParams(title = "Cancelled upload", filePath = "test_media.jpg")
            )
        }

        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        assertIs<RequestExecutionErrorReason.CancellationError>(
            (result as WpRequestResult.RequestExecutionFailed<*>).reason
        )
    }

    @Test
    fun `a CancellationException thrown synchronously by an upload callback is mapped to CancellationError`() =
        runTest {
            // The executor runs inside UniFFI's callback scope. A `CancellationException` thrown
            // synchronously by a client callback must NOT be rethrown — UniFFI would treat it as an
            // unexpected error and panic, surfacing as an uncaught `InternalException` out of
            // `WpApiClient.request`. It must instead be classified as `CancellationError` so `request`
            // still returns a result. This guards against "fixing" the executor's catch back to a
            // `throw` (the usual Kotlin idiom), which would reintroduce that crash path.
            val throwingUploadListener = object : WpRequestExecutor.UploadListener {
                override fun onProgressUpdate(uploadedBytes: Long, totalBytes: Long) {
                    // Not relevant to this test.
                }

                override fun onUploadStarted(cancellableUpload: WpRequestExecutor.CancellableUpload) {
                    throw CancellationException("cancelled synchronously from callback")
                }
            }

            val executor = WpRequestExecutor(
                httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
                networkAvailabilityProvider = NetworkAvailabilityProvider { true },
                fileResolver = ClasspathFileResolver(),
                uploadListener = throwingUploadListener
            )

            val apiClient = WpApiClient(
                wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
                authProvider = WpAuthenticationProvider.none(),
                requestExecutor = executor
            )

            val result = apiClient.request { requestBuilder ->
                requestBuilder.media().create(
                    params = MediaCreateParams(title = "Cancelled upload", filePath = "test_media.jpg")
                )
            }

            assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
            assertIs<RequestExecutionErrorReason.CancellationError>(
                (result as WpRequestResult.RequestExecutionFailed<*>).reason
            )
        }

    @Test
    fun `cancelling the enclosing coroutine propagates cancellation instead of a RequestExecutionFailed`() =
        runBlocking {
            // Stall the response body so the request is in-flight when we cancel. Cancelling the
            // enclosing coroutine must surface as cancellation out of `WpApiClient.request`, not a
            // returned `RequestExecutionFailed`. That guarantee is caller-side: `request` catches only
            // `WpApiException`, so a `CancellationException` propagates instead of being mapped to a
            // result. (The blocking `call.execute()` isn't interruptible, so the executor's own
            // `catch (CancellationException)` is never reached by coroutine cancellation — this test
            // covers the caller-side half of the contract.)
            mockWebServer.enqueue(
                MockResponse()
                    .setBody("[]")
                    .setBodyDelay(500, TimeUnit.MILLISECONDS)
            )

            val loggedErrors = mutableListOf<String>()
            val executor = WpRequestExecutor(
                httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
                networkAvailabilityProvider = NetworkAvailabilityProvider { true }
            )
            val apiClient = WpApiClient(
                wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
                authProvider = WpAuthenticationProvider.none(),
                requestExecutor = executor,
                errorLogger = RequestErrorLogger { message -> loggedErrors.add(message) }
            )

            var observedResult: WpRequestResult<*>? = null
            val job = launch(Dispatchers.IO) {
                observedResult = apiClient.request { requestBuilder ->
                    requestBuilder.users().listWithEditContext(params = UserListParams())
                }
            }

            // Block until the request reaches the server, guaranteeing the executor is mid-flight
            // before we cancel.
            assertNotNull(
                mockWebServer.takeRequest(5, TimeUnit.SECONDS),
                "request should have reached the mock server"
            )
            job.cancelAndJoin()

            // A cancelled coroutine must not produce a returned result, and cancellation must not be
            // logged as a request failure.
            assertNull(observedResult, "cancellation should not yield a returned WpRequestResult")
            assertTrue(loggedErrors.isEmpty(), "cancellation should not be logged as an error: $loggedErrors")
        }

    @Test
    fun `refused connection is mapped to ConnectionError`() = runTest {
        // Reserve a port with a throwaway server, then shut it down so nothing is
        // listening — a connection to it is refused (ConnectException). That must
        // classify as ConnectionError, not the generic HttpError and not
        // NonExistentSiteError (which is reserved for DNS failures).
        val closedServer = MockWebServer()
        closedServer.start()
        val refusedUrl = closedServer.url("/wp-json")
        closedServer.shutdown()

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true }
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(refusedUrl.toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        val result = apiClient.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }

        assertIs<WpRequestResult.RequestExecutionFailed<*>>(result)
        assertIs<RequestExecutionErrorReason.ConnectionError>(
            (result as WpRequestResult.RequestExecutionFailed<*>).reason
        )
    }

    @Test
    fun `a file that fails while its bytes are read is mapped to MediaFileUnreadable`() = runTest {
        // A file that passes the pre-upload check (`exists()`/`isFile`/`canRead()`) but then fails
        // when OkHttp reads its bytes to stream the body — e.g. deleted after the check, or a storage
        // read error. This drives the whole executor -> caller chain that `MediaFileRequestBodyTest`
        // only covers in isolation: `MediaFileRequestBody` tags the read failure as
        // `MediaFileUnreadableException`, `executeRequestSafely` catches it *before* the generic
        // `IOException`, and it surfaces as `WpRequestResult.MediaFileUnreadable` carrying the path.
        // Guards against dropping/reordering that catch, which would re-drop it to `GenericError`.
        // The body write fails before any response is read, so an enqueued response is never used —
        // it only keeps a regression that lets the request complete from blocking on an empty queue.
        mockWebServer.enqueue(MockResponse())

        val filePath = "/tmp/wp-rs-test/gone-after-check.jpg"
        // Lies about being present and readable so `canBeUploaded()` passes, but points at a path with
        // no file, so okio's `file.source()` throws `FileNotFoundException` exactly where OkHttp
        // streams the request body — a deterministic, uid-independent stand-in for a mid-upload read
        // failure (a genuine one is a non-deterministic TOCTOU race against the pre-upload check).
        val unreadableFileResolver = object : FileResolver {
            override fun getFile(path: String): File = object : File(filePath) {
                override fun exists() = true
                override fun isFile() = true
                override fun canRead() = true
                override fun length() = 8L
            }
        }

        val executor = WpRequestExecutor(
            httpClient = WpHttpClient.CustomOkHttpClient(OkHttpClient()),
            networkAvailabilityProvider = NetworkAvailabilityProvider { true },
            fileResolver = unreadableFileResolver
        )

        val apiClient = WpApiClient(
            wpOrgSiteApiRootUrl = URI(mockWebServer.url("/wp-json").toString()).toURL(),
            authProvider = WpAuthenticationProvider.none(),
            requestExecutor = executor
        )

        val result = apiClient.request { requestBuilder ->
            requestBuilder.media().create(
                params = MediaCreateParams(title = "Unreadable upload", filePath = filePath)
            )
        }

        assertIs<WpRequestResult.MediaFileUnreadable<*>>(result)
        assertEquals(filePath, (result as WpRequestResult.MediaFileUnreadable<*>).filePath)
    }

    /**
     * Resolves media fixtures (e.g. `test_media.jpg`) from the test classpath so uploads can be
     * built without touching the real filesystem layout.
     */
    private class ClasspathFileResolver : FileResolver {
        override fun getFile(path: String): File? =
            this::class.java.classLoader?.getResource(path)?.file?.let { File(it) }
    }
}
