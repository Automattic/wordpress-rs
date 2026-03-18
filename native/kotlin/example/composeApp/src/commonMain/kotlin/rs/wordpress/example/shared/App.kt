package rs.wordpress.example.shared

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import org.koin.compose.koinInject
import rs.wordpress.api.kotlin.WpApiClient
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.di.createWpApiClient
import rs.wordpress.example.shared.di.createWpComApiClient
import rs.wordpress.example.shared.di.createWpService
import rs.wordpress.example.shared.ui.components.ErrorMessage
import rs.wordpress.example.shared.ui.applicationpasswords.ApplicationPasswordListScreen
import rs.wordpress.example.shared.ui.comments.CommentListScreen
import rs.wordpress.example.shared.ui.login.LoginScreen
import rs.wordpress.example.shared.ui.media.MediaListScreen
import rs.wordpress.example.shared.ui.menulocations.MenuLocationListScreen
import rs.wordpress.example.shared.ui.navigations.NavigationListScreen
import rs.wordpress.example.shared.ui.navmenuitems.NavMenuItemListScreen
import rs.wordpress.example.shared.ui.navmenus.NavMenuListScreen
import rs.wordpress.example.shared.ui.plugins.PluginListScreen
import rs.wordpress.example.shared.ui.postcollection.PostCollectionScreen
import rs.wordpress.example.shared.ui.posts.PostListByTypeScreen
import rs.wordpress.example.shared.ui.postmetadatacollection.PostMetadataCollectionScreen
import rs.wordpress.example.shared.ui.posttypes.PostTypesScreen
import rs.wordpress.example.shared.ui.search.SearchScreen
import rs.wordpress.example.shared.ui.settings.SiteSettingsScreen
import rs.wordpress.example.shared.ui.site.SitePostType
import rs.wordpress.example.shared.ui.site.SiteScreen
import rs.wordpress.example.shared.ui.site.SiteTaxonomy
import rs.wordpress.example.shared.ui.site.SiteViewModel
import rs.wordpress.example.shared.ui.sitehealth.SiteHealthScreen
import rs.wordpress.example.shared.ui.stresstest.StressTestScreen
import rs.wordpress.example.shared.ui.terms.TermListByTypeScreen
import rs.wordpress.example.shared.ui.themes.ThemeListScreen
import rs.wordpress.example.shared.ui.users.UserListScreen
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen
import rs.wordpress.example.shared.ui.wpcom.WpComBotConversationsScreen
import rs.wordpress.example.shared.ui.wpcom.WpComMeScreen
import rs.wordpress.example.shared.ui.wpcom.WpComSiteScreen
import rs.wordpress.example.shared.ui.wpcom.WpComSupportConversationsScreen
import uniffi.wp_api.PostEndpointType
import uniffi.wp_api.TermEndpointType
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.WpService

@Composable
fun App(authenticationEnabled: Boolean, authenticateSite: (String, onSuccess: () -> Unit, onError: (String) -> Unit) -> Unit, authenticateWpCom: (() -> Unit)?) {
    val cache = koinInject<WordPressApiCache>()
    val navController = rememberNavController()

    // State to hold the current post type slug for navigation
    var currentPostTypeSlug by remember { mutableStateOf("post") }
    var currentWpService by remember { mutableStateOf<WpService?>(null) }
    var currentAccount by remember { mutableStateOf<Account.SelfHostedSite?>(null) }
    var currentWpComClient by remember { mutableStateOf<WpComApiClient?>(null) }
    var currentApiClient by remember { mutableStateOf<WpApiClient?>(null) }
    var currentSiteViewModel by remember { mutableStateOf<SiteViewModel?>(null) }
    var currentPostType by remember { mutableStateOf<SitePostType?>(null) }
    var currentTaxonomy by remember { mutableStateOf<SiteTaxonomy?>(null) }

    MaterialTheme {
        NavHost(navController, startDestination = "welcome") {
            composable("welcome") {
                WelcomeScreen(
                    authenticationEnabled,
                    onLoginClicked = {
                        navController.navigate("login")
                    },
                    onSiteClicked = { account ->
                        when (account) {
                            is Account.SelfHostedSite -> {
                                currentAccount = account
                                currentWpService = createWpService(account, cache)
                                val apiClient = createWpApiClient(account)
                                currentApiClient = apiClient
                                currentSiteViewModel = SiteViewModel(apiClient)
                                navController.navigate("site")
                            }
                            is Account.WpCom -> {
                                if (account.siteApiRoot.isNotEmpty()) {
                                    // Site-specific WP.com account with a blog ID
                                    currentWpService = createWpService(account, cache)
                                    val apiClient = createWpApiClient(account)
                                    currentApiClient = apiClient
                                    currentSiteViewModel = SiteViewModel(apiClient)
                                    navController.navigate("site")
                                } else {
                                    currentWpComClient = createWpComApiClient(account)
                                    navController.navigate("wpcom_site")
                                }
                            }
                        }
                    }
                )
            }
            composable("login") {
                if (authenticationEnabled) {
                    LoginScreen(
                        authenticateSite = authenticateSite,
                        authenticateWpCom = authenticateWpCom,
                        onBackClicked = { navController.popBackStack() }
                    )
                } else {
                    throw IllegalStateException("Authentication is disabled")
                }
            }
            composable("site") {
                val viewModel = currentSiteViewModel
                if (viewModel == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                SiteScreen(
                    viewModel = viewModel,
                    onPostTypeClicked = { postType ->
                        currentPostType = postType
                        navController.navigate("postlistbytype")
                    },
                    onCommentsClicked = { navController.navigate("comments") },
                    onMediaClicked = { navController.navigate("media") },
                    onTaxonomyClicked = { taxonomy ->
                        currentTaxonomy = taxonomy
                        navController.navigate("termlistbytype")
                    },
                    onNavigationsClicked = { navController.navigate("navigations") },
                    onNavMenusClicked = { navController.navigate("navmenus") },
                    onNavMenuItemsClicked = { navController.navigate("navmenuitems") },
                    onMenuLocationsClicked = { navController.navigate("menulocations") },
                    onApplicationPasswordsClicked = { navController.navigate("applicationpasswords") },
                    onUsersClicked = { navController.navigate("users") },
                    onPluginsClicked = { navController.navigate("plugins") },
                    onPostCollectionClicked = { navController.navigate("postcollection") },
                    onPostTypesClicked = { navController.navigate("posttypes") },
                    onThemesClicked = { navController.navigate("themes") },
                    onSiteSettingsClicked = { navController.navigate("sitesettings") },
                    onSearchClicked = { navController.navigate("search") },
                    onSiteHealthClicked = { navController.navigate("sitehealth") },
                    onStressTestClicked = { navController.navigate("stresstest") },
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("users") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                UserListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("plugins") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                PluginListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("termlistbytype") {
                val apiClient = currentApiClient
                val taxonomy = currentTaxonomy
                if (apiClient == null || taxonomy == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                TermListByTypeScreen(
                    apiClient = apiClient,
                    termEndpointType = TermEndpointType.Custom(taxonomy.restBase),
                    title = taxonomy.name,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("postlistbytype") {
                val apiClient = currentApiClient
                val postType = currentPostType
                if (apiClient == null || postType == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                PostListByTypeScreen(
                    apiClient = apiClient,
                    postEndpointType = PostEndpointType.Custom(postType.restBase),
                    title = postType.name,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("comments") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                CommentListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("media") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                MediaListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("themes") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                ThemeListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("sitesettings") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                SiteSettingsScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("search") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                SearchScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("sitehealth") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                SiteHealthScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("stresstest") {
                val wpService = currentWpService
                val account = currentAccount
                if (wpService == null || account == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                StressTestScreen(
                    wpService = wpService,
                    account = account,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("postcollection") {
                val wpService = currentWpService
                if (wpService == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                PostCollectionScreen(
                    wpService = wpService,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("posttypes") {
                val wpService = currentWpService
                if (wpService == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                PostTypesScreen(
                    wpService = wpService,
                    onBackClicked = { navController.popBackStack() },
                    onPostTypeClicked = { postTypeSlug ->
                        currentPostTypeSlug = postTypeSlug
                        navController.navigate("postmetadatacollection")
                    }
                )
            }
            composable("postmetadatacollection") {
                val wpService = currentWpService
                if (wpService == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                PostMetadataCollectionScreen(
                    wpService = wpService,
                    postTypeSlug = currentPostTypeSlug,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("navigations") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                NavigationListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("navmenus") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                NavMenuListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("navmenuitems") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                NavMenuItemListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("menulocations") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                MenuLocationListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("applicationpasswords") {
                val apiClient = currentApiClient
                if (apiClient == null) {
                    ErrorMessage("No site connected")
                    return@composable
                }
                ApplicationPasswordListScreen(
                    apiClient = apiClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_site") {
                WpComSiteScreen(
                    onMeClicked = { navController.navigate("wpcom_me") },
                    onSupportConversationsClicked = { navController.navigate("wpcom_support") },
                    onBotConversationsClicked = { navController.navigate("wpcom_bots") },
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_me") {
                val wpComClient = currentWpComClient
                if (wpComClient == null) {
                    ErrorMessage("No WordPress.com account connected")
                    return@composable
                }
                WpComMeScreen(
                    wpComApiClient = wpComClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_support") {
                val wpComClient = currentWpComClient
                if (wpComClient == null) {
                    ErrorMessage("No WordPress.com account connected")
                    return@composable
                }
                WpComSupportConversationsScreen(
                    wpComApiClient = wpComClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_bots") {
                val wpComClient = currentWpComClient
                if (wpComClient == null) {
                    ErrorMessage("No WordPress.com account connected")
                    return@composable
                }
                WpComBotConversationsScreen(
                    wpComApiClient = wpComClient,
                    onBackClicked = { navController.popBackStack() }
                )
            }
        }
    }
}
