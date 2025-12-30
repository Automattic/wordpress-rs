package rs.wordpress.example.shared

import androidx.compose.material.MaterialTheme
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
import rs.wordpress.example.shared.ui.site.SiteScreen
import rs.wordpress.example.shared.ui.stresstest.StressTestScreen
import rs.wordpress.example.shared.ui.users.UserListScreen
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen

@Composable
fun App(authenticationEnabled: Boolean, authenticateSite: (String) -> Unit) {
    val userListViewModel = koinInject<UserListViewModel>()
    val pluginListViewModel = koinInject<PluginListViewModel>()
    val navController = rememberNavController()

    // State to hold the current post type slug for navigation
    var currentPostTypeSlug by remember { mutableStateOf("post") }

    MaterialTheme {
        NavHost(navController, startDestination = "welcome") {
            composable("welcome") {
                WelcomeScreen(
                    authenticationEnabled,
                    onLoginClicked = {
                        navController.navigate("login")
                    },
                    onSiteClicked = { authenticatedSite ->
                        userListViewModel.setAuthenticatedSite(authenticatedSite)
                        pluginListViewModel.setAuthenticatedSite(authenticatedSite)
                        navController.navigate("site")
                    }
                )
            }
            composable("login") {
                if (authenticationEnabled) {
                    LoginScreen(authenticateSite)
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
                    }
                )
            }
            composable("users") {
                UserListScreen()
            }
            composable("plugins") {
                PluginListScreen()
            }
            composable("stresstest") {
                StressTestScreen()
            }
            composable("postcollection") {
                PostCollectionScreen(
                    onBackClicked = { navController.popBackStack() }
                )
            }
            composable("posttypes") {
                PostTypesScreen(
                    onBackClicked = { navController.popBackStack() },
                    onPostTypeClicked = { postTypeSlug ->
                        currentPostTypeSlug = postTypeSlug
                        navController.navigate("postmetadatacollection")
                    }
                )
            }
            composable("postmetadatacollection") {
                PostMetadataCollectionScreen(
                    postTypeSlug = currentPostTypeSlug,
                    onBackClicked = { navController.popBackStack() }
                )
            }
        }
    }
}
