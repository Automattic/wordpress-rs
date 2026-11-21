package rs.wordpress.api.kotlin

import uniffi.wp_api.RequestContext
import uniffi.wp_api.RequestExecutor
import uniffi.wp_api.WpApiMiddleware
import uniffi.wp_api.WpNetworkRequest
import uniffi.wp_api.WpNetworkResponse
import uniffi.wp_api.WpRequestUrlLogDetail
import uniffi.wp_api.redactRequestUrlForLog

/**
 * Used in a middleware pipeline to print each request and its response to standard error, which
 * Android sends to the log. For debugging only: don't add it to a release build.
 *
 * It leaves out what grants access to an account:
 * - The value of every header that carries a credential ([CREDENTIAL_HEADERS]: `Authorization`,
 *   `Proxy-Authorization`, `Cookie`, `Set-Cookie`, `X-WP-Nonce`), matched without regard to case,
 *   on the request and on the response. The header's name is printed, with `<redacted>` in place
 *   of its value. An `Authorization` value keeps its scheme, such as `Bearer`, and nothing after it.
 * - In the URL: user name and password, the fragment, and the value of any query parameter whose
 *   name marks it as a secret, such as `token` or `password` (see `redactRequestUrlForLog`).
 * - In a JSON response body: the value of any member whose name marks it as a secret, such as
 *   `access_token` or `password` (see [redactJsonSecrets]).
 *
 * It still prints everything else as it was sent or received, which can include personal data:
 * the rest of the query, a body that isn't JSON, and every other header and member.
 * The request body is not printed.
 */
class DebugMiddleware internal constructor(
    private val printLine: (String) -> Unit
) : WpApiMiddleware {
    constructor() : this({ line -> System.err.println(line) })

    override suspend fun process(
        requestExecutor: RequestExecutor,
        response: WpNetworkResponse,
        request: WpNetworkRequest,
        context: RequestContext?
    ): WpNetworkResponse {
        debugLogLines(
            method = request.method().toString(),
            url = redactRequestUrlForLog(request.url(), WpRequestUrlLogDetail.FULL),
            requestHeaders = request.headerMap().toMap(),
            statusCode = response.statusCode,
            responseHeaders = response.responseHeaderMap.toMap(),
            body = String(response.body)
        ).forEach(printLine)
        return response
    }
}

private const val TAG = "WpDebug"

internal const val REDACTED = "<redacted>"

/** Headers whose value is a credential, in lower case. */
internal val CREDENTIAL_HEADERS = setOf(
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-wp-nonce"
)

/** Headers whose value starts with an authentication scheme, which is kept. */
private val SCHEME_HEADERS = setOf("authorization", "proxy-authorization")

/** A scheme, such as `Bearer` or `Basic`: a token of letters, digits and a few punctuation marks. */
private val AUTH_SCHEME = Regex("[A-Za-z0-9!#$%&'*+.^_`|~-]+")

/** Parts of a JSON member's name that make its value a secret, in lower case. */
private val SECRET_NAME_PARTS = listOf("token", "secret", "password")

/** JSON member names whose value is a secret, in lower case. */
private val SECRET_NAMES = setOf(
    "api_key",
    "apikey",
    "authorization",
    "cookie",
    "hmac",
    "nonce",
    "passwd",
    "pwd",
    "signature",
    "_wpnonce"
)

/** The lines [DebugMiddleware] prints for one request and its response, with credentials taken out. */
@Suppress("LongParameterList")
internal fun debugLogLines(
    method: String,
    url: String,
    requestHeaders: Map<String, List<String>>,
    statusCode: UInt,
    responseHeaders: Map<String, List<String>>,
    body: String
): List<String> = buildList {
    add("===== REQUEST =====")
    add("$method $url")
    add("--- Request Headers ---")
    addAll(headerLines(requestHeaders))
    add("===== RESPONSE =====")
    add("Status: $statusCode")
    add("--- Response Headers ---")
    addAll(headerLines(responseHeaders))
    add("--- Response Body ---")
    add(redactJsonSecrets(body))
    add("====================")
}.map { "$TAG: $it" }

private fun headerLines(headers: Map<String, List<String>>): List<String> =
    headers.flatMap { (name, values) ->
        values.map { value -> "  $name: ${redactHeaderValue(name, value)}" }
    }

/**
 * [value] as it can be printed: unchanged, unless [name] is a [CREDENTIAL_HEADERS] header. Then
 * it's [REDACTED], after the authentication scheme for an `Authorization` header.
 */
internal fun redactHeaderValue(name: String, value: String): String {
    val lowercaseName = name.lowercase()
    if (lowercaseName !in CREDENTIAL_HEADERS) {
        return value
    }
    val scheme = value.trim().substringBefore(' ', missingDelimiterValue = "")
    return if (lowercaseName in SCHEME_HEADERS && AUTH_SCHEME.matches(scheme)) {
        "$scheme $REDACTED"
    } else {
        REDACTED
    }
}

private fun isSecretName(name: String): Boolean {
    val lowercaseName = name.lowercase()
    return lowercaseName in SECRET_NAMES || SECRET_NAME_PARTS.any { lowercaseName.contains(it) }
}

/**
 * [body] with the value of each JSON member whose name marks it as a secret replaced by
 * `"<redacted>"`, at any depth. A string, number or literal value is replaced; an object or array
 * value is kept, with its own members checked the same way.
 *
 * Text that doesn't start like JSON (with `{` or `[`) is returned as it is.
 */
@Suppress("CyclomaticComplexMethod", "NestedBlockDepth", "LoopWithTooManyJumpStatements")
internal fun redactJsonSecrets(body: String): String {
    val first = body.firstOrNull { !it.isWhitespace() }
    if (first != '{' && first != '[') {
        return body
    }
    val out = StringBuilder(body.length)
    // Set after a secret member's name and its colon, until its value has been seen.
    var redactNextValue = false
    var index = 0
    while (index < body.length) {
        val char = body[index]
        when {
            char == '"' -> {
                val end = endOfString(body, index)
                val string = body.substring(index, end)
                var next = end
                while (next < body.length && body[next].isWhitespace()) next++
                val isName = next < body.length && body[next] == ':'
                when {
                    isName -> {
                        out.append(string)
                        redactNextValue = isSecretName(string.removeSurrounding("\""))
                    }
                    redactNextValue -> {
                        out.append("\"").append(REDACTED).append("\"")
                        redactNextValue = false
                    }
                    else -> out.append(string)
                }
                index = end
            }
            redactNextValue && (char == '-' || char.isLetterOrDigit()) -> {
                // A number, `true`, `false` or `null`.
                var end = index
                while (end < body.length && (body[end].isLetterOrDigit() || body[end] in "+-.")) end++
                out.append("\"").append(REDACTED).append("\"")
                redactNextValue = false
                index = end
            }
            else -> {
                if (char == '{' || char == '[') {
                    redactNextValue = false
                }
                out.append(char)
                index++
            }
        }
    }
    return out.toString()
}

/** The index just past the string that starts with the quote at [start], or the end of [text]. */
private fun endOfString(text: String, start: Int): Int {
    var index = start + 1
    while (index < text.length) {
        when (text[index]) {
            '\\' -> index += 2
            '"' -> return index + 1
            else -> index++
        }
    }
    return text.length
}
