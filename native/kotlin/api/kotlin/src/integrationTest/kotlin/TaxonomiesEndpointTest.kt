package rs.wordpress.api.kotlin

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.TaxonomyListParams
import uniffi.wp_api.TaxonomyType
import uniffi.wp_api.WpAuthenticationProvider
import kotlin.test.assertEquals

class TaxonomiesEndpointTest {
    private val testCredentials = TestCredentials.INSTANCE
    private val authProvider = WpAuthenticationProvider.staticWithUsernameAndPassword(
        username = testCredentials.adminUsername, password = testCredentials.adminPassword
    )
    private val client = WpApiClient(testCredentials.apiRootUrl, authProvider)

    @Test
    fun testTaxonomyListRequest() = runTest {
        val taxonomyList = client.request { requestBuilder ->
            requestBuilder.taxonomies().listWithEditContext(params = TaxonomyListParams())
        }.assertSuccessAndRetrieveData().data
        assert(taxonomyList.taxonomyTypes.isNotEmpty())
    }

    @Test
    fun testRetrieveCategoryTaxonomyRequest() = runTest {
        val taxonomy = client.request { requestBuilder ->
            requestBuilder.taxonomies().retrieveWithEditContext(TaxonomyType.Category)
        }.assertSuccessAndRetrieveData().data
        assertEquals("Categories", taxonomy.name)
    }

}
