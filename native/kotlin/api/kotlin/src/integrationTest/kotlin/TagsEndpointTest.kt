package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseAnyTermFieldWithEditContext
import uniffi.wp_api.TermCreateParams
import uniffi.wp_api.TermListParams
import uniffi.wp_api.TermUpdateParams
import uniffi.wp_api.TermEndpointType
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
            requestBuilder.terms()
                .listWithEditContext(TermEndpointType.Tags, params = TermListParams())
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
    }

    @Test
    fun testFilterTagListRequest() = runTest {
        val tagList = client.request { requestBuilder ->
            requestBuilder.terms().filterListWithEditContext(
                TermEndpointType.Tags,
                params = TermListParams(),
                fields = listOf(
                    SparseAnyTermFieldWithEditContext.NAME,
                    SparseAnyTermFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(tagList.isNotEmpty())
        assertNull(tagList.first().description)
    }

    @Test
    fun testRetrieveMediaRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.terms().retrieveWithEditContext(TermEndpointType.Tags, TAG_ID_100)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(tag)
    }

    @Test
    fun testFilterRetrieveTagRequest() = runTest {
        val tag = client.request { requestBuilder ->
            requestBuilder.terms().filterRetrieveWithEditContext(
                TermEndpointType.Tags,
                TAG_ID_100,
                fields = listOf(
                    SparseAnyTermFieldWithEditContext.NAME,
                    SparseAnyTermFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(tag.description)
    }

    @Test
    fun createTagRequest() = runTest {
        val createdTag = client.request { requestBuilder ->
            requestBuilder.terms()
                .create(
                    TermEndpointType.Tags,
                    TermCreateParams(name = "foo", description = "bar")
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", createdTag.name)
        assertEquals("bar", createdTag.description)
        restoreTestServer()
    }

    @Test
    fun deleteTagRequest() = runTest {
        val deletedTag = client.request { requestBuilder ->
            requestBuilder.terms().delete(TermEndpointType.Tags, termId = TAG_ID_100)
        }.assertSuccessAndRetrieveData().data
        assert(deletedTag.deleted)
        restoreTestServer()
    }

    @Test
    fun updateTagRequest() = runTest {
        val updatedTag = client.request { requestBuilder ->
            requestBuilder.terms()
                .update(
                    TermEndpointType.Tags,
                    termId = TAG_ID_100,
                    TermUpdateParams(
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
                requestBuilder.terms()
                    .retrieveWithEditContext(TermEndpointType.Tags, 9999999)
            }
        assertEquals(WpErrorCode.TERM_INVALID, result.wpErrorCode())
    }
}
