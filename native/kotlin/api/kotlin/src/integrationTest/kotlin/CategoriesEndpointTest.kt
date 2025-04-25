package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseCategoryFieldWithEditContext
import uniffi.wp_api.CategoryCreateParams
import uniffi.wp_api.CategoryListParams
import uniffi.wp_api.CategoryUpdateParams
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
            requestBuilder.categories().listWithEditContext(params = CategoryListParams())
        }.assertSuccessAndRetrieveData().data
        assert(categoryList.isNotEmpty())
    }

    @Test
    fun testFilterCategoryListRequest() = runTest {
        val categoryList = client.request { requestBuilder ->
            requestBuilder.categories().filterListWithEditContext(
                params = CategoryListParams(),
                fields = listOf(
                    SparseCategoryFieldWithEditContext.NAME,
                    SparseCategoryFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(categoryList.isNotEmpty())
        assertNull(categoryList.first().description)
    }

    @Test
    fun testRetrieveCategoryRequest() = runTest {
        val category = client.request { requestBuilder ->
            requestBuilder.categories().retrieveWithEditContext(CATEGORY_ID_59)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(category)
    }

    @Test
    fun testFilterRetrieveCategoryRequest() = runTest {
        val category = client.request { requestBuilder ->
            requestBuilder.categories().filterRetrieveWithEditContext(
                CATEGORY_ID_59,
                fields = listOf(
                    SparseCategoryFieldWithEditContext.NAME,
                    SparseCategoryFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(category.description)
    }

    @Test
    fun createCategoryRequest() = runTest {
        val createdCategory = client.request { requestBuilder ->
            requestBuilder.categories()
                .create(CategoryCreateParams(name = "foo", description = "bar"))
        }.assertSuccessAndRetrieveData().data
        assertEquals("foo", createdCategory.name)
        assertEquals("bar", createdCategory.description)
        restoreTestServer()
    }

    @Test
    fun deleteCategoryRequest() = runTest {
        val deletedCategory = client.request { requestBuilder ->
            requestBuilder.categories().delete(categoryId = CATEGORY_ID_59)
        }.assertSuccessAndRetrieveData().data
        assert(deletedCategory.deleted)
        restoreTestServer()
    }

    @Test
    fun updateCategoryRequest() = runTest {
        val updatedCategory = client.request { requestBuilder ->
            requestBuilder.categories()
                .update(
                    categoryId = CATEGORY_ID_59,
                    CategoryUpdateParams(
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
                requestBuilder.categories().retrieveWithEditContext(9999999)
            }
        assert(result.wpErrorCode() is WpErrorCode.TermInvalid)
    }

    @Test
    fun testErrorParentTermInvalid() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.categories()
                    .create(CategoryCreateParams(name = "foo", parent = 9999999))
            }
        assert(result.wpErrorCode() is WpErrorCode.TermInvalid)
    }
}
