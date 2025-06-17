package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.TemplateCreateParams
import uniffi.wp_api.TemplateListParams
import uniffi.wp_api.TemplateUpdateParams
import uniffi.wp_api.SparseTemplateFieldWithEditContext
import uniffi.wp_api.SparseTemplateTitle
import uniffi.wp_api.SparseTemplateTitleWrapper
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertEquals
import kotlin.test.assertNull

private const val TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE_SLUG: String = "single"
private const val TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE: String =
    "twentytwentyfour//$TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE_SLUG"

class TemplatesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testTemplateListRequest() = runTest {
        val templateList = client.request { requestBuilder ->
            requestBuilder.templates().listWithEditContext(params = TemplateListParams())
        }.assertSuccessAndRetrieveData().data
        assert(templateList.isNotEmpty())
    }

    @Test
    fun testFilterTemplateListRequest() = runTest {
        val templateList = client.request { requestBuilder ->
            requestBuilder.templates().filterListWithEditContext(
                params = TemplateListParams(),
                fields = listOf(
                    SparseTemplateFieldWithEditContext.AUTHOR,
                    SparseTemplateFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(templateList.isNotEmpty())
        assertNull(templateList.first().description)
    }

    @Test
    fun testRetrieveTemplateRequest() = runTest {
        val template = client.request { requestBuilder ->
            requestBuilder.templates().retrieveWithEditContext(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE)
        }.assertSuccessAndRetrieveData().data
        assertEquals(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE_SLUG, template.slug)
    }

    @Test
    fun testFilterRetrieveTemplateRequest() = runTest {
        val template = client.request { requestBuilder ->
            requestBuilder.templates().filterRetrieveWithEditContext(
                TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE,
                fields = listOf(
                    SparseTemplateFieldWithEditContext.AUTHOR,
                    SparseTemplateFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertEquals(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE_SLUG, template.slug)
        assertNull(template.description)
    }

    @Test
    fun createTemplateRequest() = runTest {
        val createdTemplate = client.request { requestBuilder ->
            requestBuilder.templates()
                .create(TemplateCreateParams(slug = "created_slug", title = "new_title"))
        }.assertSuccessAndRetrieveData().data
        assertEquals("created_slug", createdTemplate.slug)
        assertEquals(
            expected = SparseTemplateTitleWrapper.Object(
                SparseTemplateTitle(
                    raw = "new_title",
                    rendered = "new_title"
                )
            ),
            actual = createdTemplate.title
        )
        restoreTestServer()
    }

    @Test
    fun deleteTemplateRequest() = runTest {
        val deletedTemplate = client.request { requestBuilder ->
            requestBuilder.templates()
                .delete(templateId = TestCredentials.INSTANCE.integrationTestCustomTemplateId)
        }.assertSuccessAndRetrieveData().data
        assert(deletedTemplate.deleted)
        restoreTestServer()
    }

    @Test
    fun updateTemplateRequest() = runTest {
        val updatedTemplate = client.request { requestBuilder ->
            requestBuilder.templates()
                .update(
                    templateId = TestCredentials.INSTANCE.integrationTestCustomTemplateId,
                    TemplateUpdateParams(
                        description = "new_description",
                    )
                )
        }.assertSuccessAndRetrieveData().data
        assertEquals("new_description", updatedTemplate.description)
        restoreTestServer()
    }

    @Test
    fun testDeleteTemplateErrInvalidTemplate() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.templates().delete(TEMPLATE_TWENTY_TWENTY_FOUR_SINGLE)
            }
        assert(result.wpErrorCode() is WpErrorCode.InvalidTemplate)
    }
}