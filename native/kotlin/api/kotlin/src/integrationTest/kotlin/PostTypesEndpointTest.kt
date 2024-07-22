package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostType
import uniffi.wp_api.PostTypeSupports
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertNull
import kotlin.test.assertTrue

class PostTypesEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val siteUrl = testCredentials.siteUrl
    private val client = WpApiClient(
        siteUrl, wpAuthenticationFromUsernameAndPassword(
            username = testCredentials.adminUsername, password = testCredentials.adminPassword
        )
    )

    @Test
    fun testPostTypesListRequest() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().listWithEditContext()
        }
        assert(result is WpRequestSuccess)
    }

    @Test
    fun testPostTypesRetrievePost() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.POST)
        }
        assert(result is WpRequestSuccess)
        val postTypesPost = (result as WpRequestSuccess).data
        assertTrue {
            postTypesPost.supports[PostTypeSupports.Title]!!
        }
    }

    @Test
    fun testPostTypesWpFontFaceDoesNotSupportAuthor() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.WP_FONT_FACE)
        }
        assert(result is WpRequestSuccess)
        val postTypesPost = (result as WpRequestSuccess).data
        assertNull(postTypesPost.supports[PostTypeSupports.Author])
    }
}