package rs.wordpress.api.kotlin

import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.internal.tls.OkHostnameVerifier
import java.util.concurrent.TimeUnit
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.SSLSession

/**
 * Client-wide OkHttp timeouts applied by [WpHttpClient.DefaultHttpClient].
 *
 * OkHttp's built-in per-operation defaults are 10s, which is too short for slow sites or large
 * responses. These defaults are more forgiving; override any of them by passing a custom instance
 * to [WpHttpClient.DefaultHttpClient] or the [WpRequestExecutor] convenience constructor.
 */
data class HttpClientTimeouts(
    val connectTimeoutSeconds: Long = DEFAULT_CONNECT_TIMEOUT_SECONDS,
    val readTimeoutSeconds: Long = DEFAULT_READ_TIMEOUT_SECONDS,
    val writeTimeoutSeconds: Long = DEFAULT_WRITE_TIMEOUT_SECONDS,
) {
    companion object {
        const val DEFAULT_CONNECT_TIMEOUT_SECONDS = 15L
        const val DEFAULT_READ_TIMEOUT_SECONDS = 60L
        const val DEFAULT_WRITE_TIMEOUT_SECONDS = 60L
    }
}

sealed class WpHttpClient {
    abstract fun getClient(): OkHttpClient

    class DefaultHttpClient(
        private val interceptors: List<Interceptor>,
        private val timeouts: HttpClientTimeouts = HttpClientTimeouts(),
    ) : WpHttpClient() {
        private var allowedHostnames: Map<String, List<String>> = emptyMap()

        private var client: OkHttpClient = buildClient()

        fun addAllowedAlternativeNamesForHostname(hostname: String, allowedNames: List<String>) {
            // Preserve the previous records for this key
            val previousList = allowedHostnames[hostname].orEmpty()
            allowedHostnames = allowedHostnames.plus(Pair(hostname, allowedNames.plus(previousList)))
            client = buildClient()
        }

        private fun buildClient(): OkHttpClient {
            return OkHttpClient.Builder().apply {
                this@DefaultHttpClient.interceptors.forEach { addInterceptor(it) }
                hostnameVerifier(WpRequestExecutorHostnameVerifier(allowedHostnames))
                connectTimeout(timeouts.connectTimeoutSeconds, TimeUnit.SECONDS)
                readTimeout(timeouts.readTimeoutSeconds, TimeUnit.SECONDS)
                writeTimeout(timeouts.writeTimeoutSeconds, TimeUnit.SECONDS)
            }.build()
        }

        override fun getClient() = client
    }

    data class CustomOkHttpClient(private val client: OkHttpClient) : WpHttpClient() {
        override fun getClient() = client
    }
}

private class WpRequestExecutorHostnameVerifier(private val allowedHostnames: Map<String, List<String>>) :
    HostnameVerifier {
    override fun verify(hostname: String?, session: SSLSession?): Boolean {
        if (hostname == null || session == null) return false

        // Check our custom allowlist first, then fall back to default OkHttp verification
        val peerPrincipalName = session.peerPrincipal.name.replace("CN=", "")
        val customMatch = allowedHostnames[peerPrincipalName]?.contains(hostname) ?: false
        return customMatch || OkHostnameVerifier.verify(hostname, session)
    }
}
