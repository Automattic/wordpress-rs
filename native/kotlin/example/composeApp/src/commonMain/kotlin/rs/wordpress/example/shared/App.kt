package rs.wordpress.example.shared

import androidx.compose.material.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import org.koin.compose.KoinContext
import org.koin.compose.koinInject
import rs.wordpress.example.shared.ui.login.LoginScreen
import rs.wordpress.example.shared.ui.users.UserListScreen
import rs.wordpress.example.shared.ui.users.UserListViewModel
import rs.wordpress.example.shared.ui.welcome.WelcomeScreen

@Composable
fun App(authenticationEnabled: Boolean, authenticateSite: (String) -> Unit) {
    KoinContext {
        val userListViewModel = koinInject<UserListViewModel>()
        val navController = rememberNavController()

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
                            navController.navigate("users")
                        }
                    )
                }
                composable("login") {
                    authenticateSite?.let {
                        LoginScreen(authenticateSite)
                    }
                }
                composable("users") {
                    UserListScreen()
                }
            }
        }
    }
}
