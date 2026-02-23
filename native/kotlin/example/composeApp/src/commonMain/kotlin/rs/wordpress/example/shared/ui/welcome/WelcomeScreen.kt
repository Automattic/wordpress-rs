package rs.wordpress.example.shared.ui.welcome

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import org.koin.compose.koinInject
import rs.wordpress.example.TestCredentials
import rs.wordpress.example.shared.localTestSiteUrl
import uniffi.wp_mobile.Account

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@org.jetbrains.compose.ui.tooling.preview.Preview
fun WelcomeScreen(
    authenticationEnabled: Boolean,
    onLoginClicked: () -> Unit,
    onSiteClicked: (Account) -> Unit
) {
    val welcomeViewModel = koinInject<WelcomeViewModel>()
    val sites by welcomeViewModel.sites.collectAsState()

    // Refresh sites each time the screen enters the composition (e.g. after login)
    LaunchedEffect(Unit) {
        welcomeViewModel.refreshSites()
    }

    Scaffold(
        topBar = {
            TopAppBar(title = { Text("Sites") })
        },
        floatingActionButton = {
            if (authenticationEnabled) {
                FloatingActionButton(onClick = onLoginClicked) {
                    Icon(Icons.Default.Add, contentDescription = "Add new site")
                }
            }
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(paddingValues)
        ) {
            val selfHostedSites = sites.filterIsInstance<Account.SelfHostedSite>()
            val wpComSites = sites.filterIsInstance<Account.WpCom>()

            items(selfHostedSites) { account ->
                ListItem(
                    headlineContent = { Text(account.domain) },
                    modifier = Modifier.clickable { onSiteClicked(account) }
                )
            }
            if (selfHostedSites.isNotEmpty() && wpComSites.isNotEmpty()) {
                item { HorizontalDivider() }
            }
            items(wpComSites) { account ->
                ListItem(
                    headlineContent = { Text(account.username) },
                    modifier = Modifier.clickable { onSiteClicked(account) }
                )
            }
            val devUsername = TestCredentials.ADMIN_USERNAME
            val devPassword = TestCredentials.ADMIN_PASSWORD
            if (devUsername != null && devPassword != null) {
                item {
                    val testSiteUrl = localTestSiteUrl().siteUrl
                    ListItem(
                        headlineContent = { Text("Local Dev Server") },
                        supportingContent = { Text(testSiteUrl) },
                        modifier = Modifier.clickable {
                            onSiteClicked(
                                Account.SelfHostedSite(
                                    id = 0u,
                                    domain = testSiteUrl,
                                    username = devUsername,
                                    password = devPassword,
                                    siteApiRoot = "$testSiteUrl/wp-json"
                                )
                            )
                        }
                    )
                }
            }
        }
    }
}
