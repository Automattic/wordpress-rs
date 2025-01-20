package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import uniffi.wp_api.MediaUploadRequest
import uniffi.wp_api.MediaUploadRequestExecutionException
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import java.io.File

class WpRequestExecutor(
    private val okHttpClient: OkHttpClient = OkHttpClient(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) : RequestExecutor {

    override suspend fun execute(request: WpNetworkRequest): WpNetworkResponse =
        withContext(dispatcher) {
            val requestBuilder = Request.Builder().url(request.url())
            requestBuilder.method(
                request.method().toString(),
                request.body()?.contents()?.toRequestBody()
            )
            request.headerMap().toMultiMap().forEach { (key, values) ->
                values.forEach { value ->
                    requestBuilder.addHeader(key, value)
                }
            }

            okHttpClient.newCall(requestBuilder.build()).execute().use { response ->
                return@withContext WpNetworkResponse(
                    body = response.body?.bytes() ?: ByteArray(0),
                    statusCode = response.code.toUShort(),
                    headerMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap())
                )
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
            mediaUploadRequest.headerMap().toMultiMap().forEach { (key, values) ->
                values.forEach { value ->
                    requestBuilder.addHeader(key, value)
                }
            }

            okHttpClient.newCall(requestBuilder.build()).execute().use { response ->
                return@withContext WpNetworkResponse(
                    body = response.body?.bytes() ?: ByteArray(0),
                    statusCode = response.code.toUShort(),
                    headerMap = WpNetworkHeaderMap.fromMultiMap(response.headers.toMultimap())
                )
            }
        }
}
