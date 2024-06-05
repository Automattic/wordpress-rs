package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpApiParamUsersHasPublishedPosts
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpRestErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class UsersEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(siteUrl, authentication)

    @Test
    fun testUserListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = null)
        }
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testUserListRequestWithHasPublishedPostsParam() = runTest {
        val params = UserListParams(
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("post", "page"))
        )
        val result =
            client.request { requestBuilder -> requestBuilder.users().listWithEditContext(params) }
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testFilterUserListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().filterList(
                context = WpContext.EDIT,
                params = null,
                fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
            )
        }
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
        assertNull(userList.first().slug)
    }

    @Test
    fun testFilterRetrieveUserRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().filterRetrieve(
                FIRST_USER_ID,
                WpContext.EDIT,
                fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
            )
        }
        assert(result is WpRequestSuccess)
        val sparseUser = (result as WpRequestSuccess).data
        assertEquals(FIRST_USER_EMAIL, sparseUser.email)
        assertNull(sparseUser.slug)
    }

    @Test
    fun testFilterRetrieveCurrentUserRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.users().filterRetrieveMe(
                WpContext.EDIT,
                fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
            )
        }
        assert(result is WpRequestSuccess)
        val sparseUser = (result as WpRequestSuccess).data
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
        assert(result is RecognizedRestError)
        assertEquals(WpRestErrorCode.InvalidParam, (result as RecognizedRestError).error.code)
    }
}
