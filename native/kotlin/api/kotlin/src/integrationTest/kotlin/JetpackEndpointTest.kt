package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class JetpackEndpointTest {
    // These credentials are from a throwaway site
    private val client = JpApiClient(
        ParsedUrl.parse("https://solitary-warbler.jurassic.ninja/"), wpAuthenticationFromUsernameAndPassword(
            username = "demo", password = "4vhX miDF aYSd MNSL Gka5 m6WF"
        )
    )

    @Test
    fun testConnection() = runTest {
        val status = client.request { requestBuilder ->
            requestBuilder.connection().status()
        }.assertSuccessAndRetrieveData()
        assert(!status.isActive)
    }
}