package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTagFieldWithEditContext
import uniffi.wp_api.TagCreateParams
import uniffi.wp_api.TagListParams
import uniffi.wp_api.TagUpdateParams
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

private const val TAG_ID_100: Long = 100

class TagsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testTagListRequest() = runTest {
        val tagList = client.request { requestBuilder ->
            requestBuilder.tags().listWithEditContext(params = TagListParams())
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
    }

    @Test
    fun testFilterTagListRequest() = runTest {
        val tagList = client.request { requestBuilder ->
            requestBuilder.tags().filterListWithEditContext(
                params = TagListParams(),
                fields = listOf(
                    SparseTagFieldWithEditContext.NAME,
                    SparseTagFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
        assertNull(tagList.first().description)
    }

    @Test
    fun testRetrieveMediaRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.tags().retrieveWithEditContext(TAG_ID_100)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(tag)
    }

    @Test
    fun testFilterRetrieveTagRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.tags().filterRetrieveWithEditContext(
                TAG_ID_100,
                fields = listOf(
                    SparseTagFieldWithEditContext.NAME,
                    SparseTagFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(tag.description)
    }

    @Test
    fun createTagRequest() = runTest {
        val createdTag = client.request { requestBuilder ->
            requestBuilder.tags()
                .create(TagCreateParams(name = "foo", description = "bar"))
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", createdTag.name)
        assertEquals("bar", createdTag.description)
        restoreTestServer()
    }

    @Test
    fun deleteTagRequest() = runTest {
        val deletedTag = client.request { requestBuilder ->
            requestBuilder.tags().delete(tagId = TAG_ID_100)
        }.assertSuccessAndRetrieveData().data
        assert(deletedTag.deleted)
        restoreTestServer()
    }

    @Test
    fun updateTagRequest() = runTest {
        val updatedTag = client.request { requestBuilder ->
            requestBuilder.tags()
                .update(
                    tagId = TAG_ID_100,
                    TagUpdateParams(
                        name = "new_name",
                        description = "new_description",
                        slug = "new_slug"
                    )
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("new_name", updatedTag.name)
        assertEquals("new_description", updatedTag.description)
        assertEquals("new_slug", updatedTag.slug)
        restoreTestServer()
    }

    @Test
    fun testErrorTermInvalid() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.tags().retrieveWithEditContext(9999999)
            }
        assert(result.wpErrorCode() is WpErrorCode.TermInvalid)
    }
}
