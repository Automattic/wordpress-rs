package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseTemplateRevisionFieldWithEditContext
import uniffi.wp_api.TemplateRevisionListParams
import kotlin.test.assertNull

class TemplateRevisionsEndpointTest {
    private val client = defaultApiClient()
    private val templateId = TestCredentials.INSTANCE.integrationTestCustomTemplateId
    private val revisionId = TestCredentials.INSTANCE.revisionIdForCustomTemplate

    @Test
    fun testTemplateRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.templateRevisions()
                .listWithEditContext(templateId, params = TemplateRevisionListParams())
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
    }

    @Test
    fun testFilterTemplateRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.templateRevisions().filterListWithEditContext(
                templateId,
                params = TemplateRevisionListParams(),
                fields = listOf(
                    SparseTemplateRevisionFieldWithEditContext.AUTHOR,
                    SparseTemplateRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
        assertNull(revisionList.first().description)
    }

    @Test
    fun testRetrieveTemplateRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.templateRevisions()
                .retrieveWithEditContext(templateId, revisionId)
        }.assertSuccessAndRetrieveData().data
        assert(revision.slug.isNotEmpty())
    }

    @Test
    fun testFilterRetrieveTemplateRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.templateRevisions().filterRetrieveWithEditContext(
                templateId,
                revisionId,
                fields = listOf(
                    SparseTemplateRevisionFieldWithEditContext.AUTHOR,
                    SparseTemplateRevisionFieldWithEditContext.SLUG
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(revision.description)
    }

    @Test
    fun deleteTemplateRevisionRequest() = runTest {
        val deletedRevision = client.request { requestBuilder ->
            requestBuilder.templateRevisions()
                .delete(templateId, revisionId)
        }.assertSuccessAndRetrieveData().data
        assert(deletedRevision.deleted)
        restoreTestServer()
    }
}
