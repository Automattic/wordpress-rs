package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTemplatePartAutosaveFieldWithEditContext
import uniffi.wp_api.TemplatePartCreateParams
import kotlin.test.assertNull

class TemplatePartAutosavesEndpointTest {
    private val client = defaultApiClient()
    private val templatePartId = TestCredentials.INSTANCE.autosavedTemplatePartId
    private val autosaveId = TestCredentials.INSTANCE.autosaveIdForAutosavedTemplatePart

    @Test
    fun testTemplatePartAutosaveListRequest() = runTest {
        val autosaveList = client.request { requestBuilder ->
            requestBuilder.templatePartAutosaves()
                .listWithEditContext(templatePartId)
        }.assertSuccessAndRetrieveData().data
        assert(autosaveList.isNotEmpty())
    }

    @Test
    fun testFilterTemplatePartAutosaveListRequest() = runTest {
        val autosaveList = client.request { requestBuilder ->
            requestBuilder.templatePartAutosaves().filterListWithEditContext(
                templatePartId,
                fields = listOf(
                    SparseTemplatePartAutosaveFieldWithEditContext.AUTHOR,
                    SparseTemplatePartAutosaveFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(autosaveList.isNotEmpty())
        assertNull(autosaveList.first().description)
    }

    @Test
    fun testRetrieveTemplatePartAutosaveRequest() = runTest {
        val autosave = client.request { requestBuilder ->
            requestBuilder.templatePartAutosaves()
                .retrieveWithEditContext(templatePartId, autosaveId)
        }.assertSuccessAndRetrieveData().data
        assert(autosave.slug.isNotEmpty())
    }

    @Test
    fun testFilterRetrieveTemplatePartAutosaveRequest() = runTest {
        val autosave = client.request { requestBuilder ->
            requestBuilder.templatePartAutosaves().filterRetrieveWithEditContext(
                templatePartId,
                autosaveId,
                fields = listOf(
                    SparseTemplatePartAutosaveFieldWithEditContext.AUTHOR,
                    SparseTemplatePartAutosaveFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(autosave.description)
    }

    @Test
    fun createTemplatePartAutosaveRequest() = runTest {
        val createdAutosave = client.request { requestBuilder ->
            requestBuilder.templatePartAutosaves()
                .create(templatePartId, TemplatePartCreateParams(slug = "autosave_slug", content = "autosave_content"))
        }.assertSuccessAndRetrieveData().data
        assert(createdAutosave.slug.isNotEmpty())
        restoreTestServer()
    }
}
