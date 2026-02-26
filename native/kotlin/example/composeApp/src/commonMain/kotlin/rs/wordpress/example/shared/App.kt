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
import rs.wordpress.example.shared.ui.login.LoginScreen
import rs.wordpress.example.shared.ui.plugins.PluginListScreen
import rs.wordpress.example.shared.ui.plugins.PluginListViewModel
import rs.wordpress.example.shared.ui.postcollection.PostCollectionScreen
import rs.wordpress.example.shared.ui.postmetadatacollection.PostMetadataCollectionScreen
import rs.wordpress.example.shared.ui.posttypes.PostTypesScreen
import rs.wordpress.api.kotlin.WpComApiClient
import rs.wordpress.example.shared.di.createWpComApiClient
import rs.wordpress.example.shared.ui.site.SiteScreen
import rs.wordpress.example.shared.ui.stresstest.StressTestScreen
import rs.wordpress.example.shared.ui.users.UserListScreen
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen
import rs.wordpress.example.shared.ui.wpcom.WpComBotConversationsScreen
import rs.wordpress.example.shared.ui.wpcom.WpComMeScreen
import rs.wordpress.example.shared.ui.wpcom.WpComSiteScreen
import rs.wordpress.example.shared.ui.wpcom.WpComSupportConversationsScreen
import rs.wordpress.cache.kotlin.WordPressApiCache
import rs.wordpress.example.shared.di.createWpService
import uniffi.wp_mobile.Account
import uniffi.wp_mobile.WpService

@Composable
fun App(authenticationEnabled: Boolean, authenticateSite: (String) -> Unit, authenticateWpCom: (() -> Unit)?) {
    val userListViewModel = koinInject<UserListViewModel>()
    val pluginListViewModel = koinInject<PluginListViewModel>()
    val cache = koinInject<WordPressApiCache>()
    val navController = rememberNavController()

    // State to hold the current post type slug for navigation
    var currentPostTypeSlug by remember { mutableStateOf("post") }
    var currentWpService by remember { mutableStateOf<WpService?>(null) }
    var currentAccount by remember { mutableStateOf<Account.SelfHostedSite?>(null) }
    var currentWpComClient by remember { mutableStateOf<WpComApiClient?>(null) }

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
                                userListViewModel.setAccount(account)
                                pluginListViewModel.setAccount(account)
                                navController.navigate("site")
                            }
                            is Account.WpCom -> {
                                currentWpComClient = createWpComApiClient(account)
                                navController.navigate("wpcom_site")
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
                SiteScreen(
                    onUsersClicked = {
                        navController.navigate("users")
                    },
                    onPluginsClicked = {
                        navController.navigate("plugins")
                    },
                    onStressTestClicked = {
                        navController.navigate("stresstest")
                    },
                    onPostCollectionClicked = {
                        navController.navigate("postcollection")
                    },
                    onPostTypesClicked = {
                        navController.navigate("posttypes")
                    },
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("users") {
                UserListScreen(
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("plugins") {
                PluginListScreen(
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("stresstest") {
                StressTestScreen(
                    wpService = currentWpService!!,
                    account = currentAccount!!,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("postcollection") {
                PostCollectionScreen(
                    wpService = currentWpService!!,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("posttypes") {
                PostTypesScreen(
                    wpService = currentWpService!!,
                    onBackClicked = { navController.popBackStack() },
                    onPostTypeClicked = { postTypeSlug ->
                        currentPostTypeSlug = postTypeSlug
                        navController.navigate("postmetadatacollection")
                    }
                )
            }
            composable("postmetadatacollection") {
                PostMetadataCollectionScreen(
                    wpService = currentWpService!!,
                    postTypeSlug = currentPostTypeSlug,
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
                WpComMeScreen(
                    wpComApiClient = currentWpComClient!!,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_support") {
                WpComSupportConversationsScreen(
                    wpComApiClient = currentWpComClient!!,
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("wpcom_bots") {
                WpComBotConversationsScreen(
                    wpComApiClient = currentWpComClient!!,
                    onBackClicked = { navController.popBackStack() }
                )
            }
        }
    }
}
