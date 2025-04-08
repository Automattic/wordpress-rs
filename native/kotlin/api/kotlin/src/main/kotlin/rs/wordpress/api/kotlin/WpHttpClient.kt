package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.SSLSession

sealed class WpHttpClient {
    abstract fun getClient(): OkHttpClient

    class DefaultHttpClient : WpHttpClient() {
        private var client: OkHttpClient = OkHttpClient()

        private var allowedHostnames: Map<String, String> = emptyMap()

        fun addAllowedAlternativeNameForHostname(hostname: String, alternativeName: String) {
            allowedHostnames = allowedHostnames.plus(Pair(hostname, alternativeName))
            updateClient()
        }

        private fun updateClient() {
            client = client.newBuilder()
                .hostnameVerifier(WpRequestExecutorHostnameVerifier(allowedHostnames))
                .build()
        }

        override fun getClient() = client
    }

    data class CustomOkHttpClient(private val client: OkHttpClient) : WpHttpClient() {
        override fun getClient() = client
    }
}

private class WpRequestExecutorHostnameVerifier(private val allowedHostnames: Map<String, String>) :
    HostnameVerifier {
    override fun verify(hostname: String?, session: SSLSession?): Boolean =
        session?.let {
            val name = it.peerPrincipal.name.replace("CN=", "")
            name == hostname || allowedHostnames[hostname]?.let { alternativeName -> name == alternativeName } ?: false
        } ?: false
}
