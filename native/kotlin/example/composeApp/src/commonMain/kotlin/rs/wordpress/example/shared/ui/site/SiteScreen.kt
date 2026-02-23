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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
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
            // Posts
            item { SectionHeader("Posts") }
            item { NavigationItem("Post Collection", onPostCollectionClicked) }
            item { NavigationItem("Pages", onPagesClicked) }
            item { NavigationItem("Comments", onCommentsClicked) }
            item { NavigationItem("Media", onMediaClicked) }

            // Taxonomies
            item { SectionHeader("Taxonomies") }
            item { NavigationItem("Categories", onCategoriesClicked) }
            item { NavigationItem("Tags", onTagsClicked) }

            // System
            item { SectionHeader("System") }
            item { NavigationItem("Users", onUsersClicked) }
            item { NavigationItem("Plugins", onPluginsClicked) }
            item { NavigationItem("Post Types", onPostTypesClicked) }
            item { NavigationItem("Themes", onThemesClicked) }
            item { NavigationItem("Site Settings", onSiteSettingsClicked) }
            item { NavigationItem("Search", onSearchClicked) }
            item { NavigationItem("Site Health", onSiteHealthClicked) }
            item { NavigationItem("Stress Test", onStressTestClicked) }
        }
    }
}

@Composable
private fun SectionHeader(title: String) {
    Text(
        text = title,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
    )
}

@Composable
private fun NavigationItem(title: String, onClick: () -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        trailingContent = {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
        },
        modifier = Modifier.clickable(onClick = onClick)
    )
}
