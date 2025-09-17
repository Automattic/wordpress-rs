package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.ParsedUrl
import uniffi.wp_api.UniffiWpApiRequestBuilder
import uniffi.wp_api.UserListParams
import uniffi.wp_api.WpAuthenticationProvider
import uniffi.wp_api.WpOrgSiteApiUrlResolver
import uniffi.wp_api.parseAsUsersRequestListWithEditContextResponse
import kotlin.test.assertEquals

class ManualParserTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val requestExecutor by lazy { WpRequestExecutor() }

    @Test
    fun testUserListManualRequestAndParsing() = runTest {
        val requestBuilder = UniffiWpApiRequestBuilder(
            apiUrlResolver = WpOrgSiteApiUrlResolver(apiRootUrl = ParsedUrl(testCredentials.apiRootUrl)),
            authProvider
        )
        val userListRequest = requestBuilder.users().listWithEditContext(UserListParams())
        val userListResponse = requestExecutor.execute(userListRequest, null)
        val userList = parseAsUsersRequestListWithEditContextResponse(userListResponse).data
        assertEquals(NUMBER_OF_USERS, userList.count())
    }
}
