package rs.wordpress.api.kotlin

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DebugMiddlewareTest {

    @Test
    fun theRequestsCredentialsAreNotPrinted() {
        val output = lines(
            requestHeaders = mapOf(
                "authorization" to listOf("Bearer $SECRET"),
                "Proxy-Authorization" to listOf("Basic $SECRET"),
                "Cookie" to listOf("wordpress_logged_in=$SECRET"),
                "X-WP-Nonce" to listOf(SECRET),
                "accept" to listOf("application/json")
            )
        ).joinToString("\n")

        assertFalse(output, output.contains(SECRET))
        // The name is still printed, with the scheme and a marker in place of the value.
        assertTrue(output, output.contains("  authorization: Bearer <redacted>"))
        assertTrue(output, output.contains("  Proxy-Authorization: Basic <redacted>"))
        assertTrue(output, output.contains("  Cookie: <redacted>"))
        assertTrue(output, output.contains("  X-WP-Nonce: <redacted>"))
        assertTrue(output, output.contains("  accept: application/json"))
    }

    @Test
    fun theResponsesCookiesAreNotPrinted() {
        val output = lines(
            responseHeaders = mapOf("SET-COOKIE" to listOf("session=$SECRET; HttpOnly"))
        ).joinToString("\n")

        assertFalse(output, output.contains(SECRET))
        assertTrue(output, output.contains("  SET-COOKIE: <redacted>"))
    }

    @Test
    fun headerNamesAreMatchedWithoutRegardToCase() {
        listOf("Authorization", "AUTHORIZATION", "aUtHoRiZaTiOn").forEach { name ->
            assertEquals("Bearer <redacted>", redactHeaderValue(name, "Bearer $SECRET"))
        }
    }

    @Test
    fun anAuthorizationValueWithoutASchemeIsRedactedWhole() {
        assertEquals("<redacted>", redactHeaderValue("Authorization", SECRET))
        assertEquals("<redacted>", redactHeaderValue("Authorization", "\"$SECRET\" more"))
    }

    @Test
    fun secretsInAJsonBodyAreNotPrinted() {
        val body = """
            {"access_token":"$SECRET","token_type":"bearer","blog_id":"1",
             "nested":{"password" : "$SECRET","name":"Example"},
             "items":[{"refresh_token":"$SECRET"}],"expires_token":12345,
             "message":"has \"quotes\" and a password: in it"}
        """.trimIndent()

        val output = lines(body = body).joinToString("\n")

        assertFalse(output, output.contains(SECRET))
        assertFalse(output, output.contains("12345"))
        assertTrue(output, output.contains("\"access_token\":\"<redacted>\""))
        assertTrue(output, output.contains("\"password\" : \"<redacted>\""))
        assertTrue(output, output.contains("\"blog_id\":\"1\""))
        assertTrue(output, output.contains("\"name\":\"Example\""))
        assertTrue(output, output.contains("""has \"quotes\" and a password: in it"""))
    }

    @Test
    fun aBodyThatIsNotJsonIsPrintedAsItIs() {
        val body = "<html><body>Not found</body></html>"
        assertEquals(body, redactJsonSecrets(body))
    }

    @Test
    fun theRestOfTheRequestIsStillPrinted() {
        val output = lines()

        assertEquals(
            listOf(
                "WpDebug: ===== REQUEST =====",
                "WpDebug: GET https://example.com/wp-json/wp/v2/posts?per_page=1",
                "WpDebug: --- Request Headers ---",
                "WpDebug: ===== RESPONSE =====",
                "WpDebug: Status: 200",
                "WpDebug: --- Response Headers ---",
                "WpDebug: --- Response Body ---",
                "WpDebug: []",
                "WpDebug: ===================="
            ),
            output
        )
    }

    private fun lines(
        requestHeaders: Map<String, List<String>> = emptyMap(),
        responseHeaders: Map<String, List<String>> = emptyMap(),
        body: String = "[]"
    ) = debugLogLines(
        method = "GET",
        url = "https://example.com/wp-json/wp/v2/posts?per_page=1",
        requestHeaders = requestHeaders,
        statusCode = 200u,
        responseHeaders = responseHeaders,
        body = body
    )

    companion object {
        private const val SECRET = "not-a-real-token"
    }
}
