package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.GlobalStylesRevisionListParams
import uniffi.wp_api.SparseGlobalStylesRevisionFieldWithEditContext
import kotlin.test.assertNotNull
import kotlin.test.assertNull

class GlobalStylesRevisionsEndpointTest {
    private val client = defaultApiClient()
    private val globalStylesId = TestCredentials.INSTANCE.globalStylesId
    private val revisionId = TestCredentials.INSTANCE.revisionIdForGlobalStylesId

    @Test
    fun testGlobalStylesRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.globalStylesRevisions()
                .listWithEditContext(globalStylesId, params = GlobalStylesRevisionListParams())
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
    }

    @Test
    fun testFilterGlobalStylesRevisionListRequest() = runTest {
        val revisionList = client.request { requestBuilder ->
            requestBuilder.globalStylesRevisions().filterListWithEditContext(
                globalStylesId,
                params = GlobalStylesRevisionListParams(),
                fields = listOf(
                    SparseGlobalStylesRevisionFieldWithEditContext.AUTHOR,
                    SparseGlobalStylesRevisionFieldWithEditContext.DATE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(revisionList.isNotEmpty())
        assertNull(revisionList.first().settings)
    }

    @Test
    fun testRetrieveGlobalStylesRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.globalStylesRevisions()
                .retrieveWithEditContext(globalStylesId, revisionId)
        }.assertSuccessAndRetrieveData().data
        assertNotNull(revision)
    }

    @Test
    fun testFilterRetrieveGlobalStylesRevisionRequest() = runTest {
        val revision = client.request { requestBuilder ->
            requestBuilder.globalStylesRevisions().filterRetrieveWithEditContext(
                globalStylesId,
                revisionId,
                fields = listOf(
                    SparseGlobalStylesRevisionFieldWithEditContext.AUTHOR,
                    SparseGlobalStylesRevisionFieldWithEditContext.DATE
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(revision.settings)
    }
}
