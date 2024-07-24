package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostType
import uniffi.wp_api.PostTypeCapabilities
import uniffi.wp_api.PostTypeSupports
import uniffi.wp_api.WpRestErrorCode
import uniffi.wp_api.wpAuthenticationFromUsernameAndPassword
import kotlin.test.assertFalse
import kotlin.test.assertNull

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
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Post)
        }
        assert(result is WpRequestSuccess)
        val postTypesPost = (result as WpRequestSuccess).data
        assert(postTypesPost.supports[PostTypeSupports.Title]!!)
        assertFalse(postTypesPost.capabilities[PostTypeCapabilities.EditPosts]!!.isEmpty())
    }

    @Test
    fun testPostTypesWpFontFaceDoesNotSupportAuthor() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.WpFontFace)
        }
        assert(result is WpRequestSuccess)
        val postTypesPost = (result as WpRequestSuccess).data
        assertNull(postTypesPost.supports[PostTypeSupports.Author])
    }

    @Test
    fun testPostTypesErrTypeInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Custom("does_not_exist"))
        }
        assert(result is RecognizedRestError)
        assert((result as RecognizedRestError).error.code is WpRestErrorCode.TypeInvalid)
    }
}