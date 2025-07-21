package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttp
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import uniffi.wp_api.InvalidSslErrorReason
import uniffi.wp_api.MediaUploadRequest
import uniffi.wp_api.MediaUploadRequestExecutionException
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseCertificate
import java.io.File
import java.net.NoRouteToHostException
import java.net.UnknownHostException
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLPeerUnverifiedException

const val USER_AGENT_HEADER_NAME = "User-Agent"

class WpRequestExecutor(
    private val httpClient: WpHttpClient = WpHttpClient.DefaultHttpClient(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
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
            request.headerMap().toMap().forEach { (key, values) ->
                values.forEach { value ->
                    requestBuilder.addHeader(key, value)
                }
            }
            requestBuilder.addHeader(
                USER_AGENT_HEADER_NAME,
                uniffi.wp_api.defaultUserAgent("kotlin-okhttp/${OkHttp.VERSION}")
            )

            val urlRequest = requestBuilder.build()

            try {
                httpClient.getClient().newCall(urlRequest).execute().use { response ->
                    return@withContext WpNetworkResponse(
                        body = response.body?.bytes() ?: ByteArray(0),
                        statusCode = response.code.toUShort(),
                        responseHeaderMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap()),
                        requestUrl = request.url(),
                        requestHeaderMap = request.headerMap()
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
            }
        }

    override suspend fun uploadMedia(mediaUploadRequest: MediaUploadRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val requestBuilder = Request.Builder().url(mediaUploadRequest.url())
            val multipartBodyBuilder = MultipartBody.Builder()
                .setType(MultipartBody.FORM)
            mediaUploadRequest.mediaParams().forEach { (k, v) ->
                multipartBodyBuilder.addFormDataPart(k, v)
            }
            val file = File(mediaUploadRequest.filePath())
            if (!file.exists() || !file.isFile || !file.canRead()) {
                throw MediaUploadRequestExecutionException.MediaFileNotFound(mediaUploadRequest.filePath())
            }
            multipartBodyBuilder.addFormDataPart(
                name = "file",
                filename = file.name,
                body = file.asRequestBody(mediaUploadRequest.fileContentType().toMediaType())
            )
            requestBuilder.method(
                method = mediaUploadRequest.method().toString(),
                body = multipartBodyBuilder.build()
            )
            mediaUploadRequest.headerMap().toMap().forEach { (key, values) ->
                values.forEach { value ->
                    requestBuilder.addHeader(key, value)
                }
            }

            httpClient.getClient().newCall(requestBuilder.build()).execute().use { response ->
                return@withContext WpNetworkResponse(
                    body = response.body?.bytes() ?: ByteArray(0),
                    statusCode = response.code.toUShort(),
                    responseHeaderMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap()),
                    requestUrl = mediaUploadRequest.url(),
                    requestHeaderMap = mediaUploadRequest.headerMap()
                )
            }
        }

    override suspend fun sleep(millis: ULong) {
        delay(millis.toLong())
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

@Suppress("UNUSED_PARAMETER")
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
    val newConnection = requestUrl.toUrl().openConnection() as HttpsURLConnection
    newConnection.setHostnameVerifier { _, _ -> return@setHostnameVerifier true }
    newConnection.connect()

    // Certificate is parsed by the Rust shared implementation.
    val certificates = newConnection.serverCertificates.map { parseCertificate(it.encoded) }
    return RequestExecutionErrorReason.InvalidSslError(
        reason = InvalidSslErrorReason.CertificateNotValidForName(
            hostname = requestUrl.host,
            presentedHostnames = listOfNotNull(certificates.first()?.commonName())
        )
    )
}

private fun requestExecutionFailedWith(reason: RequestExecutionErrorReason) =
    RequestExecutionException.RequestExecutionFailed(
        statusCode = null,
        redirects = null,
        reason = reason
    )
