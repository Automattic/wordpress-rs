package rs.wordpress.api.kotlin

import okhttp3.OkHttpClient
import okhttp3.tls.HandshakeCertificates
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.SSLSession

sealed class WpHttpClient {
    abstract fun getClient(): OkHttpClient

    class DefaultHttpClient : WpHttpClient() {
        private var client: OkHttpClient = OkHttpClient()

        private var allowedHostnames: List<String> = emptyList()

        fun addAllowedAlternativeNameForHostname(alternativeName: String, hostname: String) {
            allowedHostnames = allowedHostnames.plus(alternativeName).plus(hostname)
            updateClient()
        }

        private fun updateClient() {
            val clientCertificates = HandshakeCertificates.Builder()
                .addPlatformTrustedCertificates()
                .addInsecureHost(allowedHostnames.first())
                .build()

            client = client.newBuilder()
                .hostnameVerifier(WpRequestExecutorHostnameVerifier(allowedHostnames))
                .sslSocketFactory(
                    clientCertificates.sslSocketFactory(),
                    clientCertificates.trustManager
                )
                .build()
        }

        override fun getClient() = client
    }

    data class CustomOkHttpClient(private val client: OkHttpClient) : WpHttpClient() {
        override fun getClient() = client
    }
}

private class WpRequestExecutorHostnameVerifier(private val allowedHostnames: List<String>) :
    HostnameVerifier {
    override fun verify(p0: String?, p1: SSLSession?): Boolean {
        return allowedHostnames.contains(p0)
    }
}
