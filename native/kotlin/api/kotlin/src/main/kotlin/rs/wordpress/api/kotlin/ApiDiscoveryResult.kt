package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.ParseUrlException
import java.net.URL

sealed class ApiDiscoveryResult {
    data class Success(val success: AutoDiscoveryAttemptSuccess) : ApiDiscoveryResult()
    data class FailureParseSiteUrl(
        val error: ParseUrlException
    ) : ApiDiscoveryResult()
    data class FailureFindApiRoot(
        val parsedSiteUrl: URL,
        val findApiRootFailure: FindApiRootFailure
    ) : ApiDiscoveryResult()
    data class FailureFetchAndParseApiRoot(
        val parsedSiteUrl: URL,
        val apiRootUrl: URL,
        val fetchAndParseApiRootFailure: FetchAndParseApiRootFailure
    ) : ApiDiscoveryResult()
}
