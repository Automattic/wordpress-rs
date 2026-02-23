package rs.wordpress.example.shared.ui.site

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import org.jetbrains.compose.ui.tooling.preview.Preview

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@Preview
fun SiteScreen(
    onUsersClicked: () -> Unit,
    onPluginsClicked: () -> Unit,
    onStressTestClicked: () -> Unit,
    onPostCollectionClicked: () -> Unit,
    onPostTypesClicked: () -> Unit,
    onCategoriesClicked: () -> Unit,
    onTagsClicked: () -> Unit,
    onPagesClicked: () -> Unit,
    onCommentsClicked: () -> Unit,
    onMediaClicked: () -> Unit,
    onThemesClicked: () -> Unit,
    onSiteSettingsClicked: () -> Unit,
    onSearchClicked: () -> Unit,
    onSiteHealthClicked: () -> Unit,
    onBackClicked: () -> Unit = {}
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Site") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(paddingValues)
        ) {
            item {
                ListItem(
                    headlineContent = { Text("Users") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onUsersClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Plugins") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onPluginsClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Categories") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onCategoriesClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Tags") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onTagsClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Pages") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onPagesClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Comments") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onCommentsClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Media") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onMediaClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Themes") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onThemesClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Site Settings") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onSiteSettingsClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Search") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onSearchClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Site Health") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onSiteHealthClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Post Collection") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onPostCollectionClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Post Types") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onPostTypesClicked)
                )
            }
            item {
                ListItem(
                    headlineContent = { Text("Stress Test") },
                    trailingContent = {
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    },
                    modifier = Modifier.clickable(onClick = onStressTestClicked)
                )
            }
        }
    }
}
