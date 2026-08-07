package rs.wordpress.example.shared.ui.components

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import rs.wordpress.api.kotlin.WpRequestResult
import uniffi.wp_api.RequestExecutionErrorReason
import uniffi.wp_api.WpErrorCode

fun <T> WpRequestResult<T>.errorDescription(): String = when (this) {
    is WpRequestResult.Success -> ""
    is WpRequestResult.WpError ->
        "$errorMessage (${errorCode.displayName()})"
    is WpRequestResult.RequestExecutionFailed ->
        reason.description()
    is WpRequestResult.InvalidHttpStatusCode ->
        "Unexpected HTTP status: $statusCode"
    is WpRequestResult.ResponseParsingError ->
        "Failed to parse response: $reason"
    is WpRequestResult.SiteUrlParsingError -> "Invalid site URL: $reason"
    is WpRequestResult.MediaFileNotFound -> "File not found: $filePath"
    is WpRequestResult.UnknownError ->
        "Unknown error (HTTP $statusCode)"
}

private fun WpErrorCode.displayName(): String = when (this) {
    is WpErrorCode.CustomException -> v1
    else -> this::class.simpleName ?: "Unknown"
}

private fun RequestExecutionErrorReason.description(): String = when (this) {
    is RequestExecutionErrorReason.DeviceIsOfflineError -> "Device is offline: $errorMessage"
    is RequestExecutionErrorReason.HttpTimeoutError -> "Request timed out"
    is RequestExecutionErrorReason.InvalidSslError -> "SSL error: $reason"
    is RequestExecutionErrorReason.NonExistentSiteError -> errorMessage ?: "Site not found"
    is RequestExecutionErrorReason.ConnectionError -> "Could not connect to the server: $reason"
    is RequestExecutionErrorReason.HttpAuthenticationRequiredError -> "Authentication required for $hostname"
    is RequestExecutionErrorReason.HttpAuthenticationRejectedError -> "Authentication rejected for $hostname"
    is RequestExecutionErrorReason.HttpForbiddenError -> "Access forbidden for $hostname"
    is RequestExecutionErrorReason.MisconfiguredHttpAuthenticationError -> "HTTP authentication misconfigured: $issue"
    is RequestExecutionErrorReason.MisconfiguredRateLimitError -> "Rate limit misconfigured"
    is RequestExecutionErrorReason.CancellationError -> "Request cancelled"
    is RequestExecutionErrorReason.HttpError -> "HTTP error: $reason"
    is RequestExecutionErrorReason.GenericError -> errorMessage
}

@Composable
fun ErrorMessage(message: String, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize().padding(16.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = message,
            color = MaterialTheme.colorScheme.error,
            style = MaterialTheme.typography.bodyLarge
        )
    }
}

@Composable
fun EmptyState(message: String, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier.fillMaxSize().padding(16.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(
            text = message,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodyLarge
        )
    }
}
