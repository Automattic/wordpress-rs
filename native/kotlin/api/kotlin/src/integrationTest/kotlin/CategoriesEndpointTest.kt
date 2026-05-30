package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.TermEndpointType
import uniffi.wp_api.SparseAnyTermFieldWithEditContext
import uniffi.wp_api.TermCreateParams
import uniffi.wp_api.TermListParams
import uniffi.wp_api.TermUpdateParams
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull

private const val CATEGORY_ID_48: Long = 48
private const val CATEGORY_ID_59: Long = 59

class CategoriesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testCategoryListRequest() = runTest {
        val categoryList = client.request { requestBuilder ->
            requestBuilder.terms()
                .listWithEditContext(TermEndpointType.Categories, params = TermListParams())
        }.assertSuccessAndRetrieveData().data
        assert(categoryList.isNotEmpty())
    }

    @Test
    fun testFilterCategoryListRequest() = runTest {
        val categoryList = client.request { requestBuilder ->
            requestBuilder.terms().filterListWithEditContext(
                TermEndpointType.Categories,
                params = TermListParams(),
                fields = listOf(
                    SparseAnyTermFieldWithEditContext.NAME,
                    SparseAnyTermFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(categoryList.isNotEmpty())
        assertNull(categoryList.first().description)
    }

    @Test
    fun testRetrieveCategoryRequest() = runTest {
        val category = client.request { requestBuilder ->
            requestBuilder.terms()
                .retrieveWithEditContext(TermEndpointType.Categories, CATEGORY_ID_59)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(category)
    }

    @Test
    fun testFilterRetrieveCategoryRequest() = runTest {
        val category = client.request { requestBuilder ->
            requestBuilder.terms().filterRetrieveWithEditContext(
                TermEndpointType.Categories,
                termId = CATEGORY_ID_59,
                fields = listOf(
                    SparseAnyTermFieldWithEditContext.NAME,
                    SparseAnyTermFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(category.description)
    }

    @Test
    fun createCategoryRequest() = runTest {
        val createdCategory = client.request { requestBuilder ->
            requestBuilder.terms()
                .create(
                    TermEndpointType.Categories,
                    TermCreateParams(name = "foo", description = "bar")
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", createdCategory.name)
        assertEquals("bar", createdCategory.description)
        restoreTestServer()
    }

    @Test
    fun deleteCategoryRequest() = runTest {
        val deletedCategory = client.request { requestBuilder ->
            requestBuilder.terms()
                .delete(TermEndpointType.Categories, termId = CATEGORY_ID_59)
        }.assertSuccessAndRetrieveData().data
        assert(deletedCategory.deleted)
        restoreTestServer()
    }

    @Test
    fun updateCategoryRequest() = runTest {
        val updatedCategory = client.request { requestBuilder ->
            requestBuilder.terms()
                .update(
                    TermEndpointType.Categories,
                    termId = CATEGORY_ID_59,
                    TermUpdateParams(
                        name = "new_name",
                        description = "new_description",
                        slug = "new_slug",
                        parent = CATEGORY_ID_48
                    )
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("new_name", updatedCategory.name)
        assertEquals("new_description", updatedCategory.description)
        assertEquals("new_slug", updatedCategory.slug)
        assertEquals(CATEGORY_ID_48, updatedCategory.parent)
        restoreTestServer()
    }

    @Test
    fun testErrorTermInvalid() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.terms()
                    .retrieveWithEditContext(
                        TermEndpointType.Categories,
                        termId = 9999999,
                    )
            }
        assertEquals(WpErrorCode.TERM_INVALID, result.wpErrorCode())
    }

    @Test
    fun testErrorParentTermInvalid() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.terms()
                    .create(
                        TermEndpointType.Categories,
                        TermCreateParams(name = "foo", parent = 9999999)
                    )
            }
        assertEquals(WpErrorCode.TERM_INVALID, result.wpErrorCode())
    }
}
