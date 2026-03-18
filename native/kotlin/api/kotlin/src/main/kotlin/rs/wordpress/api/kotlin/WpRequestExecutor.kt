package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import okhttp3.Call
import okhttp3.HttpUrl
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttp
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
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
import java.net.ConnectException
import java.net.NoRouteToHostException
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import javax.net.ssl.SSLException
import javax.net.ssl.SSLPeerUnverifiedException

const val USER_AGENT_HEADER_NAME = "User-Agent"
private const val HTTP_PROBE_TIMEOUT_MS = 5000
private const val DEFAULT_HTTPS_PORT = 443
private const val DEFAULT_HTTP_PORT = 80

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
     * Uses [WpHttpClient.DefaultHttpClient] internally with the provided interceptors.
     */
    @JvmOverloads
    constructor(
        interceptors: List<Interceptor> = listOf(),
        networkAvailabilityProvider: NetworkAvailabilityProvider,
        dispatcher: CoroutineDispatcher = Dispatchers.IO,
        fileResolver: FileResolver = DefaultFileResolver(),
        uploadListener: UploadListener? = null
    ) : this(
        httpClient = WpHttpClient.DefaultHttpClient(interceptors),
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
                    val requestBody = file.asRequestBody(mimeType.toMediaType())
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
    @Suppress("ThrowsCount", "TooGenericExceptionCaught", "SwallowedException")
    private fun executeRequestSafely(
        urlRequest: Request,
        requestUrl: String,
        requestMethod: RequestMethod,
        requestHeaderMap: WpNetworkHeaderMap,
        notifyUploadListener: Boolean = false
    ): WpNetworkResponse {
        try {
            val call = httpClient.getClient().newCall(urlRequest)

            // Notify upload listener if this is an upload request
            if (notifyUploadListener) {
                uploadListener?.onUploadStarted(CancellableCall(call))
            }

            return call.execute().use { response ->
                WpNetworkResponse(
                    body = response.body.bytes(),
                    statusCode = response.code.toUShort(),
                    responseHeaderMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap()),
                    requestUrl = requestUrl,
                    requestMethod = requestMethod,
                    requestHeaderMap = requestHeaderMap
                )
            }
        } catch (e: SSLPeerUnverifiedException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.invalidSSLError(e, urlRequest.url),
                requestUrl,
                requestMethod,
            )
        } catch (e: SSLException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.sslException(e, urlRequest.url)
            )
        } catch (e: UnknownHostException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.unknownHost(e, networkAvailabilityProvider),
                requestUrl,
                requestMethod,
            )
        } catch (e: NoRouteToHostException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.noRouteToHost(e),
                requestUrl,
                requestMethod,
            )
        } catch (e: ConnectException) {
            val cause = e.cause
            if (cause is SSLException) {
                throw requestExecutionFailedWith(
                    RequestExecutionErrorReason.sslException(cause, urlRequest.url),
                    requestUrl,
                    requestMethod,
                )
            }
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.connectException(e, urlRequest.url),
                requestUrl,
                requestMethod,
            )
        } catch (e: SocketTimeoutException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.HttpTimeoutError,
                requestUrl,
                requestMethod,
            )
        } catch (e: Exception) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.GenericError(
                    errorMessage = e.localizedMessage ?: e.toString()
                ),
                requestUrl,
                requestMethod,
            )
        }
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
        return RequestExecutionErrorReason.DeviceIsOfflineError(
            errorMessage = e.localizedMessage ?: "No internet connection"
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
private fun RequestExecutionErrorReason.Companion.connectException(
    e: ConnectException, // To avoid `SwallowedException` from Detekt
    requestUrl: HttpUrl
): RequestExecutionErrorReason {
    // Connection was refused. If the original URL was HTTPS, check whether the site
    // is reachable via HTTP — if so, the site doesn't support HTTPS.
    if (requestUrl.scheme != "https") {
        return RequestExecutionErrorReason.HttpError(
            reason = "Connection failed: ${e.localizedMessage}"
        )
    }
    return try {
        // If the original URL used the default HTTPS port (443), probe on the default
        // HTTP port (80). Otherwise, keep the custom port and just change the scheme.
        val httpUrlBuilder = requestUrl.newBuilder().scheme("http")
        if (requestUrl.port == DEFAULT_HTTPS_PORT) {
            httpUrlBuilder.port(DEFAULT_HTTP_PORT)
        }
        val httpUrl = httpUrlBuilder.build()
        val probeClient = OkHttpClient.Builder()
            .connectTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .readTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .build()
        val request = Request.Builder().url(httpUrl).head().build()
        probeClient.newCall(request).execute().close()
        // Site responded over HTTP — it just doesn't support HTTPS
        RequestExecutionErrorReason.HttpsNotSupportedError
    } catch (ex: Exception) {
        // HTTP also failed — site is genuinely unreachable
        RequestExecutionErrorReason.HttpError(
            reason = "Connection failed: ${e.localizedMessage}"
        )
    }
}

@Suppress("UNUSED_PARAMETER", "TooGenericExceptionCaught", "SwallowedException")
private fun RequestExecutionErrorReason.Companion.sslException(
    e: SSLException, // To avoid `SwallowedException` from Detekt
    requestUrl: HttpUrl
): RequestExecutionErrorReason {
    // Try to re-connect with relaxed TLS settings to inspect the server's certificate.
    // If this also fails, the server likely doesn't support HTTPS at all.
    return try {
        val trustAllManager = object : javax.net.ssl.X509TrustManager {
            override fun checkClientTrusted(chain: Array<java.security.cert.X509Certificate>, authType: String) = Unit
            override fun checkServerTrusted(chain: Array<java.security.cert.X509Certificate>, authType: String) = Unit
            override fun getAcceptedIssuers(): Array<java.security.cert.X509Certificate> = arrayOf()
        }
        val sslContext = javax.net.ssl.SSLContext.getInstance("TLS")
        sslContext.init(null, arrayOf(trustAllManager), null)

        val probeClient = OkHttpClient.Builder()
            .sslSocketFactory(sslContext.socketFactory, trustAllManager)
            .hostnameVerifier(javax.net.ssl.HostnameVerifier { _, _ -> true })
            .connectTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .readTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .build()
        val request = Request.Builder().url(requestUrl).head().build()
        val response = probeClient.newCall(request).execute()
        try {
            val handshake = response.handshake
            if (handshake == null) {
                RequestExecutionErrorReason.HttpsNotSupportedError
            } else {
                val certificates = handshake.peerCertificates.map { cert -> parseCertificate(cert.encoded) }
                RequestExecutionErrorReason.InvalidSslError(
                    reason = InvalidSslErrorReason.CertificateNotValidForName(
                        hostname = requestUrl.host,
                        presentedHostnames = listOfNotNull(certificates.first()?.commonName())
                    )
                )
            }
        } finally {
            response.close()
        }
    } catch (ex: Exception) {
        // Re-connection also failed — server doesn't support HTTPS
        RequestExecutionErrorReason.HttpsNotSupportedError
    }
}

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
        val trustAllManager = object : javax.net.ssl.X509TrustManager {
            override fun checkClientTrusted(chain: Array<java.security.cert.X509Certificate>, authType: String) = Unit
            override fun checkServerTrusted(chain: Array<java.security.cert.X509Certificate>, authType: String) = Unit
            override fun getAcceptedIssuers(): Array<java.security.cert.X509Certificate> = arrayOf()
        }
        val sslContext = javax.net.ssl.SSLContext.getInstance("TLS")
        sslContext.init(null, arrayOf(trustAllManager), null)

        val probeClient = OkHttpClient.Builder()
            .sslSocketFactory(sslContext.socketFactory, trustAllManager)
            .hostnameVerifier(javax.net.ssl.HostnameVerifier { _, _ -> true })
            .connectTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .readTimeout(HTTP_PROBE_TIMEOUT_MS.toLong(), java.util.concurrent.TimeUnit.MILLISECONDS)
            .build()
        val request = Request.Builder().url(requestUrl).head().build()
        val response = probeClient.newCall(request).execute()
        try {
            // Certificate is parsed by the Rust shared implementation.
            val handshake = response.handshake
            if (handshake != null) {
                val certificates = handshake.peerCertificates.map { cert -> parseCertificate(cert.encoded) }
                RequestExecutionErrorReason.InvalidSslError(
                    reason = InvalidSslErrorReason.CertificateNotValidForName(
                        hostname = requestUrl.host,
                        presentedHostnames = listOfNotNull(certificates.first()?.commonName())
                    )
                )
            } else {
                RequestExecutionErrorReason.InvalidSslError(
                    reason = InvalidSslErrorReason.CertificateNotValidForName(
                        hostname = requestUrl.host,
                        presentedHostnames = emptyList()
                    )
                )
            }
        } finally {
            response.close()
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
