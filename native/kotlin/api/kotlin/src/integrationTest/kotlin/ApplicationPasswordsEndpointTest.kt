package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ApplicationPasswordUuid
import kotlin.test.assertEquals

class ApplicationPasswordsEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val client = defaultApiClient()

    @Test
    fun testApplicationPasswordListRequest() = runTest {
        val applicationPasswordList = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().listWithEditContext(FIRST_USER_ID)
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            ApplicationPasswordUuid(testCredentials.adminPasswordUuid),
            applicationPasswordList.first().uuid
        )
    }

    @Test
    fun testApplicationPasswordRetrieveRequest() = runTest {
        val uuid = ApplicationPasswordUuid(testCredentials.adminPasswordUuid)
        val applicationPasswordList = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().retrieveWithEditContext(FIRST_USER_ID, uuid)
        }.assertSuccessAndRetrieveData().data
        assertEquals(uuid, applicationPasswordList.uuid)
    }

    @Test
    fun testApplicationPasswordRetrieveCurrentRequest() = runTest {
        val applicationPasswordList = client.request { requestBuilder ->
            requestBuilder.applicationPasswords().retrieveCurrentWithEditContext()
        }.assertSuccessAndRetrieveData().data
        assertEquals(
            ApplicationPasswordUuid(testCredentials.adminPasswordUuid),
            applicationPasswordList.uuid
        )
    }
}
