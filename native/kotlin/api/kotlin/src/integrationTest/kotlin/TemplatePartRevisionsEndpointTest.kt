package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTemplatePartRevisionFieldWithEditContext
import uniffi.wp_api.TemplatePartRevisionListParams
import kotlin.test.assertNull

class TemplatePartRevisionsEndpointTest {
    private val client = defaultApiClient()
    private val templatePartId = TestCredentials.INSTANCE.integrationTestCustomTemplatePartId
    private val revisionId = TestCredentials.INSTANCE.revisionIdForCustomTemplatePart

    @Test
    fun testTemplatePartRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.templatePartRevisions()
                .listWithEditContext(templatePartId, params = TemplatePartRevisionListParams())
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
    }

    @Test
    fun testFilterTemplatePartRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.templatePartRevisions().filterListWithEditContext(
                templatePartId,
                params = TemplatePartRevisionListParams(),
                fields = listOf(
                    SparseTemplatePartRevisionFieldWithEditContext.AUTHOR,
                    SparseTemplatePartRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
        assertNull(revisionList.first().description)
    }

    @Test
    fun testRetrieveTemplatePartRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.templatePartRevisions()
                .retrieveWithEditContext(templatePartId, revisionId)
        }.assertSuccessAndRetrieveData().data
        assert(revision.slug.isNotEmpty())
    }

    @Test
    fun testFilterRetrieveTemplatePartRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.templatePartRevisions().filterRetrieveWithEditContext(
                templatePartId,
                revisionId,
                fields = listOf(
                    SparseTemplatePartRevisionFieldWithEditContext.AUTHOR,
                    SparseTemplatePartRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(revision.description)
    }

    @Test
    fun deleteTemplatePartRevisionRequest() = runTest {
        val deletedRevision = client.request { requestBuilder ->
            requestBuilder.templatePartRevisions()
                .delete(templatePartId, revisionId)
        }.assertSuccessAndRetrieveData().data
        assert(deletedRevision.deleted)
        restoreTestServer()
    }
}
