package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.Request
import rs.wordpress.api.kotlin.WpRequestResult.RequestExecutionFailed
import uniffi.wp_api.AutoDiscoveryUniffiResult
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.SslCertificateInfo
import uniffi.wp_api.UniffiWpLoginClient
import javax.net.ssl.SSLPeerUnverifiedException

class WpLoginClient(
    private val requestExecutor: RequestExecutor = WpRequestExecutor(),
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO
) {
    private val internalClient by lazy {
        UniffiWpLoginClient(requestExecutor)
    }

    suspend fun apiDiscovery(siteUrl: String): AutoDiscoveryUniffiResult = withContext(dispatcher) {
        try {
            internalClient.apiDiscovery(siteUrl)
        } catch (e: SSLPeerUnverifiedException) {

            val reason = RequestExecutionErrorReason.InvalidSslError(
                siteCertificate = null,
                certificateChain = emptyList(),
                errorMessage = "Foo bar",
                suggestedAction = null
            )

            throw RequestExecutionFailed(
                statusCode = null,
                redirects = null,
                reason = reason
            )
        }
    }

    suspend fun loginUrl(siteUrl: String): String? = withContext(dispatcher) {
        internalClient.apiDiscovery(siteUrl).successfulAttempt?.apiDetails()?.findApplicationPasswordsAuthenticationUrl()
    }
}
