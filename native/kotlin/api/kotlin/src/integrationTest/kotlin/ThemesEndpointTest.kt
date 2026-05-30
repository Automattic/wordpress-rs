package rs.wordpress.api.kotlin

import kotlin.test.assertEquals

import kotlinx.coroutines.test.runTest
import org.junit.jupiter.api.Test
import uniffi.wp_api.SparseThemeFieldWithEditContext
import uniffi.wp_api.ThemeListParams
import uniffi.wp_api.ThemeStylesheet
import uniffi.wp_api.WpErrorCode
import kotlin.test.assertNotNull
import kotlin.test.assertNull
private const val THEME_TWENTY_TWENTY_FIVE: String = "twentytwentyfive"

class ThemesEndpointTest {
    private val client = defaultApiClient()

    @Test
    fun testThemeListRequest() = runTest {
        val themeList = client.request { requestBuilder ->
            requestBuilder.themes().listWithEditContext(params = ThemeListParams())
        }.assertSuccessAndRetrieveData().data
        assert(themeList.isNotEmpty())
    }

    @Test
    fun testFilterThemeListRequest() = runTest {
        val themeList = client.request { requestBuilder ->
            requestBuilder.themes().filterListWithEditContext(
                params = ThemeListParams(),
                fields = listOf(
                    SparseThemeFieldWithEditContext.NAME,
                    SparseThemeFieldWithEditContext.AUTHOR
                )
            )
        }.assertSuccessAndRetrieveData().data
        assert(themeList.isNotEmpty())
        assertNull(themeList.first().description)
    }

    @Test
    fun testRetrieveThemeRequest() = runTest {
        val theme = client.request { requestBuilder ->
            requestBuilder.themes()
                .retrieveWithEditContext(ThemeStylesheet(THEME_TWENTY_TWENTY_FIVE))
        }.assertSuccessAndRetrieveData().data
        assertNotNull(theme)
    }

    @Test
    fun testFilterRetrieveThemeRequest() = runTest {
        val theme = client.request { requestBuilder ->
            requestBuilder.themes().filterRetrieveWithEditContext(
                ThemeStylesheet(THEME_TWENTY_TWENTY_FIVE),
                fields = listOf(
                    SparseThemeFieldWithEditContext.NAME,
                    SparseThemeFieldWithEditContext.STYLESHEET
                )
            )
        }.assertSuccessAndRetrieveData().data
        assertNull(theme.description)
    }

    @Test
    fun testErrorThemeNotFound() = runTest {
        val result =
            client.request { requestBuilder ->
                requestBuilder.themes()
                    .retrieveWithEditContext(ThemeStylesheet("invalid_stylesheet"))
            }
        assertEquals(WpErrorCode.THEME_NOT_FOUND, result.wpErrorCode())
    }
}
