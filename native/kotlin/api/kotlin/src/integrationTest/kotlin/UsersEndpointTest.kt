package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import uniffi.wp_api.SparseUserField
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpApiParamUsersHasPublishedPosts
import uniffi.wp_api.WpApiParamUsersWho
import uniffi.wp_api.WpContext
import uniffi.wp_api.WpRestErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword

class UsersEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val users = WpApiClient(siteUrl, authentication).users

    @Test
    fun testUserListRequest() = runTest {
        val result = users.list.withEditContext(params = null)
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testUserListRequestWithHasPublishedPostsParam() = runTest {
        // TODO: Add default values to the binding constructor from Rust
        val params = UserListParams(
            page = null,
            perPage = null,
            search = null,
            exclude = emptyList(),
            include = emptyList(),
            offset = null,
            order = null,
            orderby = null,
            slug = emptyList(),
            roles = emptyList(),
            capabilities = emptyList(),
            who = null,
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("post", "page"))
        )
        val result = users.list.withEditContext(params)
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
    }

    @Test
    fun testFilterUserListRequest() = runTest {
        val result = users.list.filter(
            WpContext.EDIT,
            params = null,
            fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
        )
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).data
        assertEquals(NUMBER_OF_USERS, userList.count())
        assertEquals(FIRST_USER_EMAIL, userList.first().email)
        assertNull(userList.first().slug)
    }

    @Test
    fun testFilterRetrieveUserRequest() = runTest {
        val result = users.retrieve.filter(
            FIRST_USER_ID,
            WpContext.EDIT,
            fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
        )
        assert(result is WpRequestSuccess)
        val sparseUser = (result as WpRequestSuccess).data
        assertEquals(FIRST_USER_EMAIL, sparseUser.email)
        assertNull(sparseUser.slug)
    }

    @Test
    fun testFilterRetrieveCurrentUserRequest() = runTest {
        val result = users.me.filter(
            WpContext.EDIT,
            fields = listOf(SparseUserField.EMAIL, SparseUserField.NAME)
        )
        assert(result is WpRequestSuccess)
        val sparseUser = (result as WpRequestSuccess).data
        assertEquals(FIRST_USER_EMAIL, sparseUser.email)
        assertNull(sparseUser.slug)
    }

    @Test
    fun testErrorUserListRequestWithHasPublishedPostsInvalidParam() = runTest {
        // TODO: Add default values to the binding constructor from Rust
        val params = UserListParams(
            page = null,
            perPage = null,
            search = null,
            exclude = emptyList(),
            include = emptyList(),
            offset = null,
            order = null,
            orderby = null,
            slug = emptyList(),
            roles = emptyList(),
            capabilities = emptyList(),
            who = null,
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("foo"))
        )
        val result = users.list.withEditContext(params)
        assert(result is RecognizedRestError)
        assertEquals(WpRestErrorCode.InvalidParam, (result as RecognizedRestError).error.code)
    }
}
