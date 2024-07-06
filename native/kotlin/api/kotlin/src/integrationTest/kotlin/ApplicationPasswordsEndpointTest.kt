package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ApplicationPasswordUuid
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals

class ApplicationPasswordsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testApplicationPasswordListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().listWithEditContext(FIRST_USER_ID)
        }
        assert(result is WpRequestSuccess)
        val applicationPasswordList = (result as WpRequestSuccess).data
        assertEquals(
            ApplicationPasswordUuid(testCredentials.adminPasswordUuid),
            applicationPasswordList.first().uuid
        )
    }

    @Test
    fun testApplicationPasswordRetrieveRequest() = runTest {
        val uuid = ApplicationPasswordUuid(testCredentials.adminPasswordUuid)
        val result = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().retrieveWithEditContext(FIRST_USER_ID, uuid)
        }
        assert(result is WpRequestSuccess)
        val applicationPasswordList = (result as WpRequestSuccess).data
        assertEquals(uuid, applicationPasswordList.uuid)
    }

    @Test
    fun testApplicationPasswordRetrieveCurrentRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().retrieveCurrentWithEditContext(FIRST_USER_ID)
        }
        assert(result is WpRequestSuccess)
        val applicationPasswordList = (result as WpRequestSuccess).data
        assertEquals(
            ApplicationPasswordUuid(testCredentials.adminPasswordUuid),
            applicationPasswordList.uuid
        )
    }
}