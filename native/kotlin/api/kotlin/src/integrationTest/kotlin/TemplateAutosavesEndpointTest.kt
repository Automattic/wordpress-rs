package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTemplateAutosaveFieldWithEditContext
import uniffi.wp_api.TemplateCreateParams
import kotlin.test.assertNull

class TemplateAutosavesEndpointTest {
    private val client = defaultApiClient()
    private val templateId = TestCredentials.INSTANCE.autosavedTemplateId
    private val autosaveId = TestCredentials.INSTANCE.autosaveIdForAutosavedTemplate

    @Test
    fun testTemplateAutosaveListRequest() = runTest {
        val autosaveList = client.request { requestBuilder ->
            requestBuilder.templateAutosaves()
                .listWithEditContext(templateId)
        }.assertSuccessAndRetrieveData().data
        assert(autosaveList.isNotEmpty())
    }

    @Test
    fun testFilterTemplateAutosaveListRequest() = runTest {
        val autosaveList = client.request { requestBuilder ->
            requestBuilder.templateAutosaves().filterListWithEditContext(
                templateId,
                fields = listOf(
                    SparseTemplateAutosaveFieldWithEditContext.AUTHOR,
                    SparseTemplateAutosaveFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(autosaveList.isNotEmpty())
        assertNull(autosaveList.first().description)
    }

    @Test
    fun testRetrieveTemplateAutosaveRequest() = runTest {
        val autosave = client.request { requestBuilder ->
            requestBuilder.templateAutosaves()
                .retrieveWithEditContext(templateId, autosaveId)
        }.assertSuccessAndRetrieveData().data
        assert(autosave.slug.isNotEmpty())
    }

    @Test
    fun testFilterRetrieveTemplateAutosaveRequest() = runTest {
        val autosave = client.request { requestBuilder ->
            requestBuilder.templateAutosaves().filterRetrieveWithEditContext(
                templateId,
                autosaveId,
                fields = listOf(
                    SparseTemplateAutosaveFieldWithEditContext.AUTHOR,
                    SparseTemplateAutosaveFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(autosave.description)
    }

    @Test
    fun createTemplateAutosaveRequest() = runTest {
        val createdAutosave = client.request { requestBuilder ->
            requestBuilder.templateAutosaves()
                .create(templateId, TemplateCreateParams(slug = "autosave_slug", content = "autosave_content"))
        }.assertSuccessAndRetrieveData().data
        assert(createdAutosave.slug.isNotEmpty())
        restoreTestServer()
    }
}
