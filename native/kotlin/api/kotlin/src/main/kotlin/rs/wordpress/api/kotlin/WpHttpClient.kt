package rs.wordpress.api.kotlin

import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.internal.tls.OkHostnameVerifier
import uniffi.wp_api.parseCertificate
import java.io.IOException
import java.security.cert.X509Certificate
import java.util.concurrent.TimeUnit
import javax.net.ssl.HostnameVerifier
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSession
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager

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

    /**
     * Returns the client to use for a request to [host]. Defaults to [getClient]; overridden by
     * [DefaultHttpClient] to route hosts that have opted out of certificate validation.
     */
    open fun getClient(host: String): OkHttpClient = getClient()

    class DefaultHttpClient(
        private val interceptors: List<Interceptor>,
        private val timeouts: HttpClientTimeouts = HttpClientTimeouts(),
    ) : WpHttpClient() {
        // Writers serialize on `configLock` so the read-modify-writes below can't lose an update
        // (Swift guards the equivalent state with an `NSLock`); the fields stay `@Volatile` so
        // readers on `Dispatchers.IO` in `WpRequestExecutor` observe each publish without locking.
        private val configLock = Any()

        @Volatile
        private var allowedHostnames: Map<String, List<String>> = emptyMap()

        @Volatile
        private var hostsWithoutCertificateValidation: Set<String> = emptySet()

        @Volatile
        private var client: OkHttpClient = buildClient()

        // Derived from `client` the first time a host opts out of certificate validation; null until
        // then. Built once and reused so it shares the strict client's ConnectionPool and Dispatcher
        // instead of leaking a new pool of idle connections and threads per opted-out host.
        @Volatile
        private var insecureClient: OkHttpClient? = null

        fun addAllowedAlternativeNamesForHostname(hostname: String, allowedNames: List<String>) {
            synchronized(configLock) {
                // Preserve the previous records for this key
                val previousList = allowedHostnames[hostname].orEmpty()
                allowedHostnames = allowedHostnames.plus(Pair(hostname, allowedNames.plus(previousList)))
                client = buildClient()
            }
        }

        /**
         * Disables TLS certificate validation entirely for [host], so any certificate it presents —
         * self-signed, expired, or issued by an untrusted root — is accepted.
         *
         * This removes the protection TLS provides and exposes the connection to man-in-the-middle
         * attacks, so only use it for hosts you control (for example a local development or staging
         * server). To instead accept an otherwise-valid certificate whose name doesn't cover the
         * host, use [addAllowedAlternativeNamesForHostname], which keeps chain validation intact.
         *
         * A redirect to a *different* host is refused rather than followed on the
         * validation-disabled client, so the bypass can't be extended to a host the caller never
         * opted in. Same-host redirects — including an `http` → `https` upgrade — still work; opt
         * every host you need to reach out explicitly.
         */
        fun disableCertificateValidation(host: String) {
            synchronized(configLock) {
                // Build the insecure client once, before publishing the host, so a reader that sees
                // the host in the set is guaranteed to also see a non-null `insecureClient`.
                if (insecureClient == null) {
                    insecureClient = buildInsecureClient()
                }
                // Store lower-cased: `HttpUrl` lower-cases the host we later match against, so a
                // caller passing "Dev.Local" would otherwise never match and silently get the
                // certificate error they opted out of.
                hostsWithoutCertificateValidation = hostsWithoutCertificateValidation.plus(host.lowercase())
            }
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

        private fun buildInsecureClient(): OkHttpClient {
            val trustManager = TrustAllX509TrustManager()
            val sslContext = SSLContext.getInstance("TLS").apply {
                init(null, arrayOf<TrustManager>(trustManager), null)
            }
            // Derive from the strict client so the ConnectionPool, Dispatcher, interceptors, and
            // timeouts are shared; override only trust and hostname verification, and add the
            // redirect guard.
            return client.newBuilder()
                .sslSocketFactory(sslContext.socketFactory, trustManager)
                .hostnameVerifier { _, _ -> true }
                // OkHttp follows redirects on the same client, so without this a redirect from an
                // opted-out host to an arbitrary one would extend the all-trusting bypass to a host
                // the caller never opted in. Refuse to leave the opt-out set. A network interceptor
                // (not an application one) sees every hop, including redirects.
                .addNetworkInterceptor { chain ->
                    val host = chain.request().url.host
                    if (host !in hostsWithoutCertificateValidation) {
                        throw IOException("Refusing to follow a redirect to $host on the validation-disabled client")
                    }
                    chain.proceed(chain.request())
                }
                .build()
        }

        override fun getClient() = client

        override fun getClient(host: String): OkHttpClient =
            if (hostsWithoutCertificateValidation.contains(host.lowercase())) insecureClient ?: client else client
    }

    data class CustomOkHttpClient(private val client: OkHttpClient) : WpHttpClient() {
        override fun getClient() = client
    }
}

private class WpRequestExecutorHostnameVerifier(private val allowedHostnames: Map<String, List<String>>) :
    HostnameVerifier {
    override fun verify(hostname: String?, session: SSLSession?): Boolean {
        if (hostname == null || session == null) return false

        // Check our custom allow-list first, then fall back to default OkHttp verification. The
        // shared `SslCertificateInfo.hostIsAllowListed` matcher (in the Rust core) decides whether
        // `hostname` was allow-listed for a name the presented certificate carries — matching the
        // Common Name *and* the SANs, case-insensitively, so a multi-RDN or SAN-only subject is
        // handled correctly (a bare `"CN=".replace` left the trailing RDNs attached and never
        // matched). The default `X509TrustManager` already validated the chain before we reach here,
        // so this only relaxes the hostname check. Skip the leaf parse when no exception is
        // configured — the common case.
        val customMatch = allowedHostnames.isNotEmpty() && certificateAllowsHost(session, hostname)
        return customMatch || OkHostnameVerifier.verify(hostname, session)
    }

    private fun certificateAllowsHost(session: SSLSession, hostname: String): Boolean {
        val leaf = session.peerCertificates.firstOrNull() as? X509Certificate
        val certInfo = leaf?.let { parseCertificate(it.encoded) }
        return certInfo?.hostIsAllowListed(host = hostname, allowList = allowedHostnames) ?: false
    }
}

/**
 * An [X509TrustManager] that accepts any certificate. Used only by
 * [WpHttpClient.DefaultHttpClient.disableCertificateValidation] to bypass TLS validation for hosts
 * the caller has explicitly opted in.
 */
private class TrustAllX509TrustManager : X509TrustManager {
    override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) = Unit
    override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) = Unit
    override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
}
