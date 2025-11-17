package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import okhttp3.Call
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttp
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
import java.net.UnknownHostException
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLPeerUnverifiedException

const val USER_AGENT_HEADER_NAME = "User-Agent"

class WpRequestExecutor(
    private val httpClient: WpHttpClient = WpHttpClient.DefaultHttpClient(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
    private val fileResolver: FileResolver = DefaultFileResolver(),
    private val uploadListener: UploadListener? = null
) : RequestExecutor {
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

            executeRequestSafely(urlRequest, request.url(), request.headerMap())
        }

    override suspend fun upload(request: WpMultipartFormRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val multipartBody = buildMultipartBody(request)
            val bodyWithProgress = wrapWithProgressTracking(multipartBody)
            val requestBuilder = Request.Builder().url(request.url())
            requestBuilder.method(request.method().toString(), bodyWithProgress)

            addRequestHeaders(requestBuilder, request.headerMap())
            val urlRequest = requestBuilder.build()

            executeRequestSafely(urlRequest, request.url(), request.headerMap(), notifyUploadListener = true)
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

    @Suppress("ThrowsCount")
    private fun executeRequestSafely(
        urlRequest: Request,
        requestUrl: String,
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
                    body = response.body?.bytes() ?: ByteArray(0),
                    statusCode = response.code.toUShort(),
                    responseHeaderMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap()),
                    requestUrl = requestUrl,
                    requestHeaderMap = requestHeaderMap
                )
            }
        } catch (e: SSLPeerUnverifiedException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.invalidSSLError(e, urlRequest.url)
            )
        } catch (e: UnknownHostException) {
            throw requestExecutionFailedWith(RequestExecutionErrorReason.unknownHost(e))
        } catch (e: NoRouteToHostException) {
            throw requestExecutionFailedWith(RequestExecutionErrorReason.noRouteToHost(e))
        } catch (e: ConnectException) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.HttpError(
                    reason = "Connection failed: ${e.localizedMessage}"
                )
            )
        } catch (e: Exception) {
            throw requestExecutionFailedWith(
                RequestExecutionErrorReason.GenericError(
                    errorMessage = e.localizedMessage ?: e.toString()
                )
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

private fun RequestExecutionErrorReason.Companion.unknownHost(e: UnknownHostException) =
    RequestExecutionErrorReason.NonExistentSiteError(
        errorMessage = e.localizedMessage,
        suggestedAction = "Check that the URL is valid and try again"
    )

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
            val certificates = newConnection.serverCertificates.map { parseCertificate(it.encoded) }
            RequestExecutionErrorReason.InvalidSslError(
                reason = InvalidSslErrorReason.CertificateNotValidForName(
                    hostname = requestUrl.host,
                    presentedHostnames = listOfNotNull(certificates.first()?.commonName())
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

private fun requestExecutionFailedWith(reason: RequestExecutionErrorReason) =
    RequestExecutionException.RequestExecutionFailed(
        statusCode = null,
        redirects = null,
        reason = reason
    )
