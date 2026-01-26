package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.PostType
import uniffi.wp_api.PostTypeCapabilities
import uniffi.wp_api.PostTypeSupports
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull

class PostTypesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testPostTypesListRequest() = runTest {
        val postTypes = client.request { requestBuilder ->
            requestBuilder.postTypes().listWithEditContext()
        }.assertSuccessAndRetrieveData().data.postTypes
        assertEquals("Posts", postTypes[PostType.Post]!!.name)
    }

    @Test
    fun testPostTypesRetrievePost() = runTest {
        val postTypesPost = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Post)
        }.assertSuccessAndRetrieveData().data
        assert(postTypesPost.supports.supports(PostTypeSupports.Title))
        assertFalse(postTypesPost.capabilities[PostTypeCapabilities.EditPosts]!!.isEmpty())
    }

    @Test
    fun testPostTypesWpFontFaceDoesNotSupportAuthor() = runTest {
        val postTypesPost = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.WpFontFace)
        }.assertSuccessAndRetrieveData().data
        assertNull(postTypesPost.supports.map[PostTypeSupports.Author])
    }

    @Test
    fun testPostTypesErrTypeInvalid() = runTest {
        val result = client.request { requestBuilder ->
            requestBuilder.postTypes().retrieveWithEditContext(PostType.Custom("does_not_exist"))
        }
        assert(result.wpErrorCode() is WpErrorCode.TypeInvalid)
    }
}
