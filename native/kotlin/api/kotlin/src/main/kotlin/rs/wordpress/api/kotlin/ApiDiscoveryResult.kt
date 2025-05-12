package rs.wordpress.api.kotlin

import uniffi.wp_api.AutoDiscoveryAttemptSuccess
import uniffi.wp_api.FetchAndParseApiRootFailure
import uniffi.wp_api.FindApiRootFailure
import uniffi.wp_api.ParseUrlException
import uniffi.wp_api.ParsedUrl

sealed class ApiDiscoveryResult {
    data class Success(val success: AutoDiscoveryAttemptSuccess) : ApiDiscoveryResult()
    data class FailureParseSiteUrl(
        val error: ParseUrlException
    ) : ApiDiscoveryResult()
    data class FailureFindApiRoot(
        val parsedSiteUrl: ParsedUrl,
        val findApiRootFailure: FindApiRootFailure
    ) : ApiDiscoveryResult()
    data class FailureFetchAndParseApiRoot(
        val parsedSiteUrl: ParsedUrl,
        val apiRootUrl: ParsedUrl,
        val fetchAndParseApiRootFailure: FetchAndParseApiRootFailure
    ) : ApiDiscoveryResult()
}
