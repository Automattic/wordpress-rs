package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.TemplatePartCreateParams
import uniffi.wp_api.TemplatePartListParams
import uniffi.wp_api.TemplatePartUpdateParams
import uniffi.wp_api.SparseTemplatePartFieldWithEditContext
import uniffi.wp_api.SparseTemplateTitle
import uniffi.wp_api.SparseTemplateTitleWrapper
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNull

private const val TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER: String =
    "twentytwentyfour//header"

class TemplatePartsEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testTemplatePartListRequest() = runTest {
        val templatePartList = client.request { requestBuilder ->
            requestBuilder.templateParts().listWithEditContext(params = TemplatePartListParams())
        }.assertSuccessAndRetrieveData().data
        assert(templatePartList.isNotEmpty())
    }

    @Test
    fun testFilterTemplatePartListRequest() = runTest {
        val templatePartList = client.request { requestBuilder ->
            requestBuilder.templateParts().filterListWithEditContext(
                params = TemplatePartListParams(),
                fields = listOf(
                    SparseTemplatePartFieldWithEditContext.AUTHOR,
                    SparseTemplatePartFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(templatePartList.isNotEmpty())
        assertNull(templatePartList.first().description)
    }

    @Test
    fun testRetrieveTemplatePartRequest() = runTest {
        val templatePart = client.request { requestBuilder ->
            requestBuilder.templateParts()
                .retrieveWithEditContext(TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER)
        }.assertSuccessAndRetrieveData().data
        assertEquals("header", templatePart.slug)
    }

    @Test
    fun testFilterRetrieveTemplatePartRequest() = runTest {
        val templatePart = client.request { requestBuilder ->
            requestBuilder.templateParts().filterRetrieveWithEditContext(
                TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER,
                fields = listOf(
                    SparseTemplatePartFieldWithEditContext.AUTHOR,
                    SparseTemplatePartFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals("header", templatePart.slug)
        assertNull(templatePart.description)
    }

    @Test
    fun createTemplatePartRequest() = runTest {
        val createdTemplatePart = client.request { requestBuilder ->
            requestBuilder.templateParts()
                .create(TemplatePartCreateParams(slug = "created_slug", title = "new_title"))
        }.assertSuccessAndRetrieveData().data
        assertEquals("created_slug", createdTemplatePart.slug)
        assertEquals(
            expected = SparseTemplateTitleWrapper.Object(
                SparseTemplateTitle(
                    raw = "new_title",
                    rendered = "new_title"
                )
            ),
            actual = createdTemplatePart.title
        )
        restoreTestServer()
    }

    @Test
    fun deleteTemplatePartRequest() = runTest {
        val deletedTemplatePart = client.request { requestBuilder ->
            requestBuilder.templateParts()
                .delete(templatePartId = TestCredentials.INSTANCE.integrationTestCustomTemplatePartId)
        }.assertSuccessAndRetrieveData().data
        assert(deletedTemplatePart.deleted)
        restoreTestServer()
    }

    @Test
    fun updateTemplatePartRequest() = runTest {
        val updatedTemplatePart = client.request { requestBuilder ->
            requestBuilder.templateParts()
                .update(
                    templatePartId = TestCredentials.INSTANCE.integrationTestCustomTemplatePartId,
                    TemplatePartUpdateParams(
                        description = "new_description",
                    )
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("new_description", updatedTemplatePart.description)
        restoreTestServer()
    }

    @Test
    fun testDeleteTemplatePartErrInvalidTemplate() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.templateParts()
                    .delete(TEMPLATE_PART_TWENTY_TWENTY_FOUR_HEADER)
            }
        assert(result.wpErrorCode() is WpErrorCode.InvalidTemplate)
    }
}
