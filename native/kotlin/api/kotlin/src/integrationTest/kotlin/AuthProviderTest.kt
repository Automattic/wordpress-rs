package rs.wordpress.api.kotlin

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ModifiableAuthenticationProvider
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpDynamicAuthenticationProvider
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals

class AuthProviderTest {
    private val testCredentials = TestCredentials.INSTANCE

    @Test
    fun testStaticAuthProvider() = runTest {
        val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
        val client = WpApiClient(testCredentials.apiRootUrl, authProvider)

        val currentUser = client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.adminUsername,
            currentUser.username
        )
    }

    @Test
    fun testDynamicAuthProvider() = runTest {
        class DynamicAuthProvider(
            var isAuthorized: Boolean = false,
            private val dispatcher: CoroutineDispatcher = Dispatchers.IO
        ): WpDynamicAuthenticationProvider {
            override fun auth(): WpAuthentication =
                if (isAuthorized) {
                    wpAuthenticationFromUsernameAndPassword(
                        username = testCredentials.adminUsername,
                        password = testCredentials.adminPassword
                    )
                } else {
                    WpAuthentication.None
                }
            override suspend fun refresh(): Boolean = withContext(dispatcher) { false }
        }

        val dynamicAuthProvider = DynamicAuthProvider()
        val authProvider = WpAuthenticationProvider.dynamic(dynamicAuthProvider)
        val client = WpApiClient(testCredentials.apiRootUrl, authProvider)

        // Assert that initial unauthorized request fails
        assert(client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        }.wpErrorCode() is WpErrorCode.Unauthorized)

        // Assert that request succeeds after setting `is_authorized = true`
        dynamicAuthProvider.isAuthorized = true
        val currentUser = client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.adminUsername,
            currentUser.username
        )
    }

    @Test
    fun testModifiableAuthProvider() = runTest {
        val modifiableAuthenticationProvider =
            ModifiableAuthenticationProvider(authentication = WpAuthentication.None)
        val authProvider = WpAuthenticationProvider.modifiable(modifiableAuthenticationProvider)
        val client = WpApiClient(testCredentials.apiRootUrl, authProvider)

        // Assert that request fails without authentication
        assert(client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        }.wpErrorCode() is WpErrorCode.Unauthorized)

        // Assert that request succeeds after authentication is modified
        modifiableAuthenticationProvider.setAuthentication(
            wpAuthenticationFromUsernameAndPassword(
                username = testCredentials.adminUsername,
                password = testCredentials.adminPassword
            )
        )
        val currentUser = client.request { requestBuilder ->
            requestBuilder.users().retrieveMeWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            testCredentials.adminUsername,
            currentUser.username
        )
    }
}