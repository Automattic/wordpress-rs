package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class JetpackEndpointTest {
    // These credentials are from a throwaway site
    private val client = JpApiClient(
        ParsedUrl.parse("https://moral-manx-partridge.jurassic.ninja/"), wpAuthenticationFromUsernameAndPassword(
            username = "demo", password = "NZKM 5vE2 4pu3 bUg8 hRIh PKR4"
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