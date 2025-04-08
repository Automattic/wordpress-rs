package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.tls.HandshakeCertificates
import uniffi.wp_api.MediaUploadRequest
import uniffi.wp_api.MediaUploadRequestExecutionException
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutionException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.parseCertificate
import java.io.File
import java.net.UnknownHostException
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.HttpsURLConnection
import javax.net.ssl.SSLPeerUnverifiedException
import javax.net.ssl.SSLSession

class WpRequestExecutor(
    private var okHttpClient: OkHttpClient = OkHttpClient(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) : RequestExecutor {

    private var allowedHostnames: List<String> = emptyList()

    fun addAllowedAlternativeNameForHostname(altname: String, hostname: String) {
        allowedHostnames = allowedHostnames.plus(altname).plus(hostname)
    }

    override suspend fun execute(request: WpNetworkRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val requestBuilder = Request.Builder().url(request.url())
            requestBuilder.method(
                request.method().toString(),
                request.body()?.contents()?.toRequestBody()
            )
            request.headerMap().toMap().forEach { (key, values) ->
                values.forEach { value ->
                    requestBuilder.addHeader(key, value)
                }
            }

            val urlRequest = requestBuilder.build()

            try {
                getClient().newCall(urlRequest).execute().use { response ->
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
                    RequestExecutionErrorReason.invalidSSLError(
                        urlRequest.url
                    )
                )
            } catch (e: UnknownHostException) {
                throw requestExecutionFailedWith(RequestExecutionErrorReason.unknownHost(e))
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
            val filePath = mediaUploadRequest.filePath()
            val file =
                WpRequestExecutor::class.java.classLoader?.getResource(filePath)?.file?.let {
                    File(
                        it
                    )
                } ?: throw MediaUploadRequestExecutionException.MediaFileNotFound(filePath)
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

            okHttpClient.newCall(requestBuilder.build()).execute().use { response ->
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

    private fun getClient(): OkHttpClient {
        if (allowedHostnames.isEmpty()) {
            return okHttpClient
        }

        val clientCertificates = HandshakeCertificates.Builder()
            .addPlatformTrustedCertificates()
            .addInsecureHost(allowedHostnames.first())
            .build()

        return okHttpClient.newBuilder()
            .hostnameVerifier(WpRequestExecutorHostnameVerifier(allowedHostnames))
            .sslSocketFactory(clientCertificates.sslSocketFactory(), clientCertificates.trustManager)
            .build()
    }
}

class WpRequestExecutorHostnameVerifier(private val allowedHostnames: List<String>) : HostnameVerifier {
    override fun verify(p0: String?, p1: SSLSession?): Boolean {
        return allowedHostnames.contains(p0)
    }
}

private fun RequestExecutionErrorReason.Companion.unknownHost(e: UnknownHostException) =
    RequestExecutionErrorReason.NonExistentSiteError(
        errorMessage = e.localizedMessage,
        suggestedAction = "Check that the URL is valid and try again"
    )

private fun RequestExecutionErrorReason.Companion.invalidSSLError(requestUrl: HttpUrl): RequestExecutionErrorReason {
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
        siteCertificate = certificates.firstOrNull(),
        certificateChain = certificates.mapNotNull { it },
        errorMessage = "Invalid certificate for domain",
        suggestedAction = null
    )
}

private fun requestExecutionFailedWith(reason: RequestExecutionErrorReason) =
    RequestExecutionException.RequestExecutionFailed(
        statusCode = null,
        redirects = null,
        reason = reason
    )