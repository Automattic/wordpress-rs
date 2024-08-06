package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseUserFieldWithEditContext
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpApiParamUsersHasPublishedPosts
import uniffi.wp_api.WpErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertEquals
import kotlin.test.assertNull

class UsersEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testUserListRequest() = runTest {
        val userList = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }.assertSuccessAndRetrieveData()
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testUserListRequestWithHasPublishedPostsParam() = runTest {
        val params = UserListParams(
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("post", "page"))
        )
        val userList =
            client.request { requestBuilder -> requestBuilder.users().listWithEditContext(params) }
                .assertSuccessAndRetrieveData()
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testFilterUserListRequest() = runTest {
        val userList = client.request { requestBuilder ->
            requestBuilder.users().filterListWithEditContext(
                params = UserListParams(),
                fields = listOf(
                    SparseUserFieldWithEditContext.EMAIL,
                    SparseUserFieldWithEditContext.NAME
                )
            )
        }.assertSuccessAndRetrieveData()
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
        assertNull(userList.first().slug)
    }

    @Test
    fun testFilterRetrieveUserRequest() = runTest {
        val sparseUser = client.request { requestBuilder ->
            requestBuilder.users().filterRetrieveWithEditContext(
                FIRST_USER_ID,
                fields = listOf(
                    SparseUserFieldWithEditContext.EMAIL,
                    SparseUserFieldWithEditContext.NAME
                )
            )
        }.assertSuccessAndRetrieveData()
        assertEquals(FIRST_USER_EMAIL, sparseUser.email)
        assertNull(sparseUser.slug)
    }

    @Test
    fun testFilterRetrieveCurrentUserRequest() = runTest {
        val sparseUser = client.request { requestBuilder ->
            requestBuilder.users().filterRetrieveMeWithEditContext(
                fields = listOf(
                    SparseUserFieldWithEditContext.EMAIL,
                    SparseUserFieldWithEditContext.NAME
                )
            )
        }.assertSuccessAndRetrieveData()
        assertEquals(FIRST_USER_EMAIL, sparseUser.email)
        assertNull(sparseUser.slug)
    }

    @Test
    fun testErrorUserListRequestWithHasPublishedPostsInvalidParam() = runTest {
        val params = UserListParams(
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("foo"))
        )
        val result =
            client.request { requestBuilder -> requestBuilder.users().listWithEditContext(params) }
        assert(result.wpErrorCode() is WpErrorCode.InvalidParam)
    }
}
