package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import uniffi.jetpack_api.JetpackNetworkResponse
import uniffi.jetpack_api.JetpackRequestExecutor
import uniffi.wp_api.WpNetworkRequest

class JpRequestExecutor(
    private val okHttpClient: OkHttpClient = OkHttpClient(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) : JetpackRequestExecutor {
    private val wpRequestExecutor by lazy {
        WpRequestExecutor(okHttpClient, dispatcher)
    }

    override suspend fun execute(request: WpNetworkRequest): JetpackNetworkResponse =
        withContext(dispatcher) {
            val innerResponse = wpRequestExecutor.execute(request)
            JetpackNetworkResponse(innerResponse)
        }
}
