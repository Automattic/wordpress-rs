package rs.wordpress.api.kotlin

import kotlinx.coroutines.delay
import okio.FileNotFoundException
import uniffi.wp_api.CancellationToken
import uniffi.wp_api.MediaUploadRequest
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import java.net.URI

class Stub(val evaluator: (WpNetworkRequest) -> Boolean, val response: WpNetworkResponse) {
    companion object {
        fun forHost(host: String, response: WpNetworkResponse): Stub {
            return Stub(evaluator = { request ->
                URI(request.url()).host == host
            }, response = response)
        }

        fun forUrl(url: String, response: WpNetworkResponse): Stub {
            return Stub({ request -> request.url() == url }, response)
        }
    }
}

class NoStubFoundException(message: String) : Exception(message)

// A class used for testing the request executor.
class MockRequestExecutor(private var stubs: List<Stub> = listOf()) : RequestExecutor {

    override suspend fun execute(request: WpNetworkRequest, cancellationToken: CancellationToken?): WpNetworkResponse {
        val stub = stubs.firstOrNull {
            it.evaluator(request)
        }

        if (stub != null) {
            return stub.response
        }

        throw NoStubFoundException("No stub found for ${request.url()}")
    }

    override suspend fun uploadMedia(mediaUploadRequest: MediaUploadRequest, cancellationToken: CancellationToken?): WpNetworkResponse {
        TODO("Not yet implemented")
    }

    override suspend fun sleep(millis: ULong) {
        delay(millis.toLong())
    }
}

val WpNetworkResponse.Companion.empty: WpNetworkResponse
    get() = WpNetworkResponse(
        ByteArray(0),
        200u,
        WpNetworkHeaderMap.empty,
        "",
        WpNetworkHeaderMap.empty
    )

val WpNetworkHeaderMap.Companion.empty: WpNetworkHeaderMap
    get() = WpNetworkHeaderMap.fromMap(mapOf())

fun WpNetworkResponse.Companion.withApiRoot(url: String): WpNetworkResponse {
    return WpNetworkResponse(
        ByteArray(0),
        200u,
        WpNetworkHeaderMap.fromMap(mapOf("Link" to "<$url>; rel=\"https://api.w.org/\"")),
        "",
        WpNetworkHeaderMap.empty
    )
}

fun WpNetworkResponse.Companion.jsonResponse(name: String): WpNetworkResponse {
    val data = {}.javaClass.getResource(name)?.readText()

    if (data == null) {
        throw FileNotFoundException("No resource found for $name")
    }

    return WpNetworkResponse(
        data.toByteArray(),
        200u,
        WpNetworkHeaderMap.fromMap(mapOf("Content-Type" to "application/json")),
        "",
        WpNetworkHeaderMap.empty
    )
}

fun WpNetworkResponse.Companion.retryResponse(delay: ULong): WpNetworkResponse {
    return WpNetworkResponse(
        ByteArray(0),
        429u,
        WpNetworkHeaderMap.fromMap(mapOf("Retry-After" to delay.toString())),
        "",
        WpNetworkHeaderMap.empty
    )
}
