package rs.wordpress.api.kotlin

import okhttp3.Interceptor
import okhttp3.OkHttpClient
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.SSLSession

sealed class WpHttpClient {
    abstract fun getClient(): OkHttpClient

    class DefaultHttpClient(
        private val interceptors: List<Interceptor>
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
                if (allowedHostnames.isNotEmpty()) {
                    hostnameVerifier(WpRequestExecutorHostnameVerifier(allowedHostnames))
                }
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
    override fun verify(hostname: String?, session: SSLSession?): Boolean =
        session?.let {
            val peerPrincipalName = it.peerPrincipal.name.replace("CN=", "")
            peerPrincipalName == hostname || allowedHostnames[peerPrincipalName]?.contains(hostname) ?: false
        } ?: false
}
