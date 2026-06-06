package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseUserFieldWithEditContext
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpApiParamUsersHasPublishedPosts
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNull

class UsersEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testUserListRequest() = runTest {
        val userList = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams())
        }.assertSuccessAndRetrieveData().data
        assertEquals(NUMBER_OF_USERS, userList.count())
    }

    @Test
    fun testUserListRequestWithHasPublishedPostsParam() = runTest {
        val params = UserListParams(
            hasPublishedPosts = WpApiParamUsersHasPublishedPosts.PostTypes(listOf("post", "page"))
        )
        val userList =
            client.request { requestBuilder -> requestBuilder.users().listWithEditContext(params) }
                .assertSuccessAndRetrieveData().data
        assertEquals(NUMBER_OF_USERS_WITH_PUBLISHED_POSTS, userList.count())
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
        }.assertSuccessAndRetrieveData().data
        assertEquals(NUMBER_OF_USERS, userList.count())
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
        }.assertSuccessAndRetrieveData().data
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
        }.assertSuccessAndRetrieveData().data
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
        assertEquals(WpErrorCode.INVALID_PARAM, result.wpErrorCode())
    }

    @Test
    fun testUserListPagination() = runTest {
        val firstPageResponse = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(params = UserListParams(perPage = 1u))
        }.assertSuccessAndRetrieveData()
        assert(firstPageResponse.data.isNotEmpty())
        val nextPageResponse = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(firstPageResponse.nextPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(nextPageResponse.data.isNotEmpty())
        val prevPageResponse = client.request { requestBuilder ->
            requestBuilder.users().listWithEditContext(nextPageResponse.prevPageParams!!)
        }.assertSuccessAndRetrieveData()
        assert(prevPageResponse.data.isNotEmpty())
    }
}
