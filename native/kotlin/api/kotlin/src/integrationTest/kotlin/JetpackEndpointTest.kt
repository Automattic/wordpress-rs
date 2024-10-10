package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class JetpackEndpointTest {
    // These credentials are from a throwaway site
    private val client = JetpackApiClient(
        ParsedUrl.parse("https://dreamily-ideal-lynx.jurassic.ninja/"), wpAuthenticationFromUsernameAndPassword(
            username = "demo", password = "kFWq bf8y YOr1 YhTZ l4Jm GoBl"
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
