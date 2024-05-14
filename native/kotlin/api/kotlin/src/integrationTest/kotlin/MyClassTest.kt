package rs.wordpress.api.kotlin

import org.junit.Assert.assertEquals
import org.junit.Test
import uniffi.wp_api.PostObject
import uniffi.wp_api.RequestMethod
import uniffi.wp_api.WpApiException
import uniffi.wp_api.WpApiHelper
import uniffi.wp_api.WpAuthentication
import uniffi.wp_api.WpRestErrorCode
import uniffi.wp_api.WpRestErrorWrapper
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertFailsWith

class MyClassTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val authentication = wpAuthenticationFromUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val networkHandler = WPNetworkHandler()
    private val requestHandler = WpRequestHandler(networkHandler)
    private val library = MyClass(networkHandler, siteUrl, authentication)

    @Test
    fun testBasicPostListRequest() {
        val request = library.postListRequest()
        assertEquals(RequestMethod.GET, request.method)
        assertEquals("$siteUrl/wp-json/wp/v2/posts?context=edit&page=1&per_page=10", request.url)
    }

    @Test
    fun testMakeBasicPostListRequest() {
        val postListResponse = library.makePostListRequest()
        val firstPost: PostObject = postListResponse.postList!!.first()
        assertEquals("Hello world!", firstPost.title?.raw)
    }

    @Test
    fun testPostListRequestForbiddenContext() {
        val unauthenticatedLibrary =
            MyClass(networkHandler, siteUrl, WpAuthentication.AuthorizationHeader("invalid_token"))
        val exception = assertFailsWith<WpApiException.RestException> {
            unauthenticatedLibrary.makePostListRequest()
        }
        val expectedStatusCode: UShort = 401u
        assertEquals(expectedStatusCode, exception.statusCode)
        assertEquals(
            WpRestErrorCode.ForbiddenContext,
            (exception.restError as WpRestErrorWrapper.Recognized).v1.code
        )
    }

    @Test
    fun testUserListRequest() {
        val usersEndpoint = WPUsersEndpoint(requestHandler, apiHelper = WpApiHelper(siteUrl, authentication))
        val result = usersEndpoint.listWithEditContext(params = null)
        assert(result is WpRequestSuccess)
        val userList = (result as WpRequestSuccess).value
        assertEquals(3, userList.count())
        assertEquals("test@example.com", userList.first().email)
    }
}
