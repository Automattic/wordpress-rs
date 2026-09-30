package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertInstanceOf
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpApiApplicationPasswordDetails
import uniffi.wp_api.WpNetworkHeaderMap
import uniffi.wp_api.WpNetworkResponse
import kotlin.test.assertEquals

// Spec Example 18
class VerifyIssuedApplicationPasswordTest {
    private val introspectUrl =
        "https://example.com/wp-json/wp/v2/users/me/application-passwords/introspect?context=edit"

    @Test
    fun testVerified() = runTest {
        val body = """{"uuid":"0b0c1a5e-8c6f-4d4b-9f3e-2f1d7c5b8a90","app_id":"","name":"App","created":"2026-09-30T01:00:00","last_used":null,"last_ip":null}"""
        assertInstanceOf<VerifyIssuedApplicationPasswordResult.Verified>(verify(200u, body))
    }

    @Test
    fun testRestNotLoggedInIsBlocked() = runTest {
        val body = """{"code":"rest_not_logged_in","message":"You are not currently logged in.","data":{"status":401}}"""
        val result = assertInstanceOf<VerifyIssuedApplicationPasswordResult.Blocked>(verify(401u, body))
        assertEquals("example.com", result.failure.hostname)
    }

    @Test
    fun testServerErrorIsOther() = runTest {
        assertInstanceOf<VerifyIssuedApplicationPasswordResult.Other>(verify(500u, ""))
    }

    private suspend fun verify(statusCode: UInt, body: String): VerifyIssuedApplicationPasswordResult {
        val response = WpNetworkResponse(
            body.toByteArray(),
            statusCode,
            WpNetworkHeaderMap.fromMap(mapOf("Content-Type" to "application/json")),
            introspectUrl,
            RequestMethod.GET,
            WpNetworkHeaderMap.empty
        )
        val client = WpLoginClient(MockRequestExecutor(listOf(Stub.forUrl(introspectUrl, response))))
        return client.verifyIssuedApplicationPassword(
            ParsedUrl.parse("https://example.com/wp-json/"),
            WpApiApplicationPasswordDetails("https://example.com", "demo", "abcd efgh")
        )
    }
}
