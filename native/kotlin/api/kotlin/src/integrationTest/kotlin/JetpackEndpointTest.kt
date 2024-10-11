package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class JetpackEndpointTest {
    // These credentials are from a throwaway site
    private val client = JetpackApiClient(
        ParsedUrl.parse("https://almost-existing-peafowl.jurassic.ninja/"), wpAuthenticationFromUsernameAndPassword(
            username = "demo", password = "81D9 k7ZP bVy5 E5Qy mRqY tt1d"
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
