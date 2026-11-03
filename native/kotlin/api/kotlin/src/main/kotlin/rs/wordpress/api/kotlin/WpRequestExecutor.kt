package rs.wordpress.api.kotlin

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import okhttp3.Call
import okhttp3.HttpUrl
import okhttp3.Interceptor
import okhttp3.MediaType
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttp
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okio.Buffer
import okio.BufferedSink
import okio.ForwardingSource
import okio.Source
import okio.source
import uniffi.wp_api.InvalidSslErrorReason
import uniffi.wp_api.RequestContext
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpMultipartFormField
import uniffi.wp_api.WpMultipartFormRequest
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseCertificate
import java.io.File
import java.io.IOException
import java.io.InterruptedIOException
import java.net.ConnectException
import java.net.NoRouteToHostException
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLPeerUnverifiedException

const val USER_AGENT_HEADER_NAME = "User-Agent"

/**
 * Provides network availability information to [WpRequestExecutor].
 *
 * On Android, implement this using [ConnectivityManager]:
 * ```
 * val provider = NetworkAvailabilityProvider {
 *     val cm = context.getSystemService(ConnectivityManager::class.java)
 *     val capabilities = cm.getNetworkCapabilities(cm.activeNetwork)
 *     capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true
 * }
 * ```
 */
fun interface NetworkAvailabilityProvider {
    fun isNetworkAvailable(): Boolean
}

class WpRequestExecutor @JvmOverloads constructor(
    private val httpClient: WpHttpClient,
    private val networkAvailabilityProvider: NetworkAvailabilityProvider,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val fileResolver: FileResolver = DefaultFileResolver(),
    private val uploadListener: UploadListener? = null
) : RequestExecutor {

    /**
     * Convenience constructor that accepts a list of OkHttp interceptors.
     * Uses [WpHttpClient.DefaultHttpClient] internally with the provided interceptors and
     * [timeouts] (defaulting to [HttpClientTimeouts]'s more forgiving values), so callers can
     * tune timeouts without building an [okhttp3.OkHttpClient] by hand.
     */
    @JvmOverloads
    constructor(
        interceptors: List<Interceptor> = listOf(),
        networkAvailabilityProvider: NetworkAvailabilityProvider,
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        fileResolver: FileResolver = DefaultFileResolver(),
        uploadListener: UploadListener? = null,
        timeouts: HttpClientTimeouts = HttpClientTimeouts(),
    ) : this(
        httpClient = WpHttpClient.DefaultHttpClient(interceptors, timeouts),
        networkAvailabilityProvider = networkAvailabilityProvider,
        dispatcher = dispatcher,
        fileResolver = fileResolver,
        uploadListener = uploadListener
    )

    override suspend fun execute(request: WpNetworkRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val requestBuilder = Request.Builder().url(request.url())
            val wpNetworkRequestBody = request.body()?.contents()?.toRequestBody()
            requestBuilder.method(
                method = request.method().toString(),
                body = if (request.method() == RequestMethod.POST) {
                    // OkHttp doesn't allow empty bodies for POST requests
                    wpNetworkRequestBody ?: "".toRequestBody()
                } else {
                    wpNetworkRequestBody
                }
            )

            addRequestHeaders(requestBuilder, request.headerMap())
            val urlRequest = requestBuilder.build()

            executeRequestSafely(urlRequest, request.url(), request.method(), request.headerMap())
        }

    override suspend fun upload(request: WpMultipartFormRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val multipartBody = buildMultipartBody(request)
            val bodyWithProgress = wrapWithProgressTracking(multipartBody)
            val requestBuilder = Request.Builder().url(request.url())
            requestBuilder.method(request.method().toString(), bodyWithProgress)

            addRequestHeaders(requestBuilder, request.headerMap())
            val urlRequest = requestBuilder.build()

            executeRequestSafely(
                urlRequest,
                request.url(),
                request.method(),
                request.headerMap(),
                notifyUploadListener = true
            )
        }

    private fun buildMultipartBody(request: WpMultipartFormRequest): MultipartBody {
        val multipartBodyBuilder = MultipartBody.Builder().setType(MultipartBody.FORM)

        request.form().forEach { field ->
            when (field) {
                is WpMultipartFormField.Text -> {
                    multipartBodyBuilder.addFormDataPart(field.name, value = field.value)
                }
                is WpMultipartFormField.File -> {
                    val fileInfo = field.file
                    val file = fileResolver.getFile(fileInfo.filePath)
                    if (file == null || !file.canBeUploaded()) {
                        throw RequestExecutionException.MediaFileNotFound(filePath = fileInfo.filePath)
                    }
                    val mimeType = fileInfo.mimeType ?: "application/octet-stream"
                    val filename = fileInfo.fileName ?: file.name
                    // A `MediaFileRequestBody` (not `file.asRequestBody`) so a failure while OkHttp
                    // reads the file mid-upload surfaces as `MediaFileUnreadable`, not `GenericError`.
                    val requestBody = MediaFileRequestBody(file, fileInfo.filePath, mimeType.toMediaType())
                    multipartBodyBuilder.addFormDataPart(
                        name = field.name,
                        filename = filename,
                        body = requestBody
                    )
                }
            }
        }

        return multipartBodyBuilder.build()
    }

    private fun wrapWithProgressTracking(multipartBody: MultipartBody): okhttp3.RequestBody {
        // Wrap the entire multipart body for progress tracking
        // This ensures progress is cumulative across all files, not per-file
        return if (uploadListener != null) {
            ProgressRequestBody(
                delegate = multipartBody,
                progressListener = object : ProgressRequestBody.ProgressListener {
                    override fun onProgress(bytesWritten: Long, contentLength: Long) {
                        uploadListener.onProgressUpdate(bytesWritten, contentLength)
                    }
                }
            )
        } else {
            multipartBody
        }
    }

    private fun addRequestHeaders(requestBuilder: Request.Builder, headerMap: WpNetworkHeaderMap) {
        headerMap.toMap().forEach { (key, values) ->
            values.forEach { value ->
                requestBuilder.addHeader(key, value)
            }
        }
        // Use header() instead of addHeader() to ensure User-Agent cannot be overridden
        requestBuilder.header(
            USER_AGENT_HEADER_NAME,
            uniffi.wp_api.defaultUserAgent("kotlin-okhttp/${OkHttp.VERSION}")
        )
    }

    // We intentionally catch all exceptions to prevent UniFFI callback crashes.
    // All exceptions are converted to proper Rust error types rather than being swallowed.
    @Suppress(
        "ThrowsCount",
        "TooGenericExceptionCaught",
        "SwallowedException",
        "CyclomaticComplexMethod",
    )
    private fun executeRequestSafely(
        urlRequest: Request,
        requestUrl: String,
        requestMethod: RequestMethod,
        requestHeaderMap: WpNetworkHeaderMap,
        notifyUploadListener: Boolean = false
    ): WpNetworkResponse {
        // Hoisted above the `try` so the `IOException` handler can distinguish a cancelled
        // request (`call.isCanceled()`) from other I/O failures.
        val call = httpClient.getClient().newCall(urlRequest)
        val reason: RequestExecutionErrorReason = try {
            // Notify upload listener if this is an upload request
            if (notifyUploadListener) {
                uploadListener?.onUploadStarted(CancellableCall(call))
            }

            return call.execute().use { response ->
                WpNetworkResponse(
                    body = response.body.bytes(),
                    statusCode = response.code.toUInt(),
                    responseHeaderMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap()),
                    requestUrl = requestUrl,
                    requestMethod = requestMethod,
                    requestHeaderMap = requestHeaderMap
                )
            }
        } catch (e: CancellationException) {
            // This is NOT coroutine cancellation: `executeRequestSafely` is non-suspending, so a
            // cancellation signal is never injected into this `try`. The only `CancellationException`
            // that can reach here is one thrown synchronously by a client callback (e.g. `onUploadStarted`).
            //
            // Deliberately NOT rethrown (the usual Kotlin idiom): this runs inside UniFFI's
            // `GlobalScope.launch` callback, where a throwable that isn't the declared
            // `RequestExecutionException` is treated as *unexpected* — UniFFI panics and it surfaces as an
            // uncaught `InternalException` out of `WpApiClient.request` (which only catches `WpApiException`).
            // Classifying it as a cancellation keeps the executor's no-throw contract and reports an honest
            // cause. Real coroutine cancellation still propagates caller-side via `suspendCancellableCoroutine`
            // in the generated await.
            RequestExecutionErrorReason.CancellationError
        } catch (e: SSLPeerUnverifiedException) {
            RequestExecutionErrorReason.invalidSSLError(e, urlRequest.url)
        } catch (e: UnknownHostException) {
            RequestExecutionErrorReason.unknownHost(e, networkAvailabilityProvider)
        } catch (e: NoRouteToHostException) {
            RequestExecutionErrorReason.noRouteToHost(e)
        } catch (e: ConnectException) {
            RequestExecutionErrorReason.ConnectionError(reason = "Connection failed: ${e.localizedMessage}")
        } catch (e: SocketTimeoutException) {
            RequestExecutionErrorReason.HttpTimeoutError
        } catch (e: InterruptedIOException) {
            // OkHttp's whole-call `callTimeout` cancels the call and throws a bare `InterruptedIOException`
            // (the superclass of `SocketTimeoutException` above); classify it as a timeout so it isn't
            // mislabeled a `CancellationError` by the `isCanceled()` check below.
            RequestExecutionErrorReason.HttpTimeoutError
        } catch (e: MediaFileUnreadableException) {
            // A file that passed the pre-upload check but failed while OkHttp read its bytes.
            // Tagged by `MediaFileRequestBody`; classified before the generic `IOException` below.
            throw RequestExecutionException.MediaFileUnreadable(filePath = e.filePath)
        } catch (e: IOException) {
            // An explicit `CancellableCall.cancel()` throws a base `IOException("Canceled")` with
            // `isCanceled() == true` — classify only that as `CancellationError` (matching Swift's
            // `URLError.cancelled`); other I/O failures stay `GenericError`.
            if (call.isCanceled()) {
                RequestExecutionErrorReason.CancellationError
            } else {
                RequestExecutionErrorReason.GenericError(
                    errorMessage = e.localizedMessage ?: e.toString()
                )
            }
        } catch (e: Exception) {
            RequestExecutionErrorReason.GenericError(
                errorMessage = e.localizedMessage ?: e.toString()
            )
        }
        throw requestExecutionFailedWith(reason, requestUrl, requestMethod)
    }

    override suspend fun sleep(millis: ULong) {
        delay(millis.toLong())
    }

    override fun cancel(context: RequestContext) {
        // No-op
    }

    private fun File.canBeUploaded() = exists() && isFile && canRead()

    /**
     * Interface for monitoring the progress and status of a media upload.
     */
    interface UploadListener {
        /**
         * Called to report the progress of the upload.
         *
         * @param uploadedBytes The number of bytes that have been uploaded so far.
         * @param totalBytes The total number of bytes to be uploaded.
         */
        fun onProgressUpdate(uploadedBytes: Long, totalBytes: Long)

        /**
         * Called when the upload starts.
         *
         * @param cancellableUpload The [CancellableUpload] object representing the upload request. This can be used
         * to cancel the upload if needed by calling [cancellableUpload.cancel].
         *
         * This method is invoked at the beginning of the upload process, allowing the caller
         * to monitor or control the upload operation.
         */
        fun onUploadStarted(cancellableUpload: CancellableUpload)
    }

    /**
     * Represents a cancellable upload operation.
     */
    interface CancellableUpload {
        /**
         * Cancels the upload operation.
         */
        fun cancel()
    }

    /**
     * Implementation of [CancellableUpload] that delegates to an OkHttp [Call].
     */
    class CancellableCall(private val call: Call) : CancellableUpload {
        override fun cancel() {
            call.cancel()
        }
    }
}

private fun RequestExecutionErrorReason.Companion.unknownHost(
    e: UnknownHostException,
    networkAvailabilityProvider: NetworkAvailabilityProvider
): RequestExecutionErrorReason {
    if (!networkAvailabilityProvider.isNetworkAvailable()) {
        // Leave the message empty so the library renders its own localized, translated
        // offline string (`device_is_offline`). The raw `UnknownHostException` text is a
        // DNS diagnostic, not something to show a user; iOS instead passes Apple's
        // localized `URLError` description here, which the library renders as-is.
        return RequestExecutionErrorReason.DeviceIsOfflineError(
            errorMessage = ""
        )
    }

    return RequestExecutionErrorReason.NonExistentSiteError(
        errorMessage = e.localizedMessage,
        suggestedAction = "Check that the URL is valid and try again"
    )
}

private fun RequestExecutionErrorReason.Companion.noRouteToHost(e: NoRouteToHostException) =
    RequestExecutionErrorReason.HttpError(
        reason = e.localizedMessage
    )

@Suppress("UNUSED_PARAMETER", "TooGenericExceptionCaught", "SwallowedException")
private fun RequestExecutionErrorReason.Companion.invalidSSLError(
    e: SSLPeerUnverifiedException, // To avoid `SwallowedException` from Detekt
    requestUrl: HttpUrl
): RequestExecutionErrorReason {
    // It's kind of weird and annoying that we need to make a second request to get
    // this data, but it doesn't seem like we can get it from the response or the
    // `SSLPeerUnverifiedException` directly.
    //
    // We spin up a new connection that'll accept any certificate. The connection will then
    // contain all the details we need for the error.
    return try {
        val newConnection = requestUrl.toUrl().openConnection() as HttpsURLConnection
        newConnection.setHostnameVerifier { _, _ -> true }
        newConnection.connect()

        try {
            // Certificate is parsed by the Rust shared implementation.
            // `serverCertificates` is leaf-first, so the site's certificate is the
            // first element. Report every hostname it presents (Common Name plus
            // SANs), not only its Common Name — a modern SAN-only certificate may
            // omit the Common Name entirely.
            val certificates = newConnection.serverCertificates.map { parseCertificate(it.encoded) }
            RequestExecutionErrorReason.InvalidSslError(
                reason = InvalidSslErrorReason.CertificateNotValidForName(
                    hostname = requestUrl.host,
                    presentedHostnames = certificates.firstOrNull()?.presentedHostnames() ?: emptyList()
                )
            )
        } finally {
            newConnection.disconnect()
        }
    } catch (ex: Exception) {
        // Fallback if certificate inspection fails due to network issues, cast failures, etc.
        // We intentionally catch Exception here as we want to return a valid error response
        // even if certificate inspection fails. The original SSL error (e parameter) is
        // preserved in the calling context. This is a best-effort attempt to get cert details.
        RequestExecutionErrorReason.InvalidSslError(
            reason = InvalidSslErrorReason.CertificateNotValidForName(
                hostname = requestUrl.host,
                presentedHostnames = emptyList()
            )
        )
    }
}

private fun requestExecutionFailedWith(
    reason: RequestExecutionErrorReason,
    requestUrl: String,
    requestMethod: RequestMethod,
) =
    RequestExecutionException.RequestExecutionFailed(
        statusCode = null,
        redirects = null,
        reason = reason,
        requestUrl = requestUrl,
        requestMethod = requestMethod,
    )

/**
 * A media file that failed while its bytes were read to stream an upload (deleted after
 * the pre-upload check, or a storage read error). Subclasses [IOException] so it flows
 * through OkHttp's request-body path to the executor, which maps it to
 * [RequestExecutionException.MediaFileUnreadable]. `internal` for unit testing.
 */
internal class MediaFileUnreadableException(
    val filePath: String,
    cause: IOException,
) : IOException(cause)

/**
 * Streams [file] as a request body, reporting a *read* (file) failure as
 * [MediaFileUnreadableException] while a *write* (socket) failure stays an ordinary
 * `IOException`. Mirrors the Swift executor's read-vs-generic split.
 */
internal class MediaFileRequestBody(
    private val file: File,
    private val filePath: String,
    private val mediaType: MediaType?,
) : RequestBody() {
    override fun contentType(): MediaType? = mediaType

    override fun contentLength(): Long = file.length()

    override fun writeTo(sink: BufferedSink) {
        val fileSource = try {
            file.source()
        } catch (e: IOException) {
            throw MediaFileUnreadableException(filePath, e)
        }
        readTaggingSource(filePath, fileSource).use { source ->
            sink.writeAll(source)
        }
    }
}

/** Wraps [delegate] so a failed read throws [MediaFileUnreadableException]. `internal` for tests. */
internal fun readTaggingSource(filePath: String, delegate: Source): Source =
    object : ForwardingSource(delegate) {
        override fun read(sink: Buffer, byteCount: Long): Long =
            try {
                super.read(sink, byteCount)
            } catch (e: IOException) {
                throw MediaFileUnreadableException(filePath, e)
            }
    }
