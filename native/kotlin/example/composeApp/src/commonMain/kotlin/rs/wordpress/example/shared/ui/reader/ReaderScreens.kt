package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.FormatSize
import androidx.compose.material3.Button
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.UriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import com.skydoves.landscapist.coil3.CoilImage
import kotlinx.coroutines.launch
import rs.wordpress.example.shared.ui.components.ErrorMessage
import rs.wordpress.example.shared.ui.components.LoadingIndicator
import uniffi.wp_mobile.ReaderAppColors
import uniffi.wp_mobile.ReaderBrand
import uniffi.wp_mobile.ReaderFont
import uniffi.wp_mobile.ReaderLink
import uniffi.wp_mobile.ReaderPageOptions
import uniffi.wp_mobile.ReaderPostContent
import uniffi.wp_mobile.ReaderTheme
import uniffi.wp_mobile.ReaderThemeColors
import uniffi.wp_mobile.readerColorFromArgb
import uniffi.wp_mobile.readerDefaultAppColors
import uniffi.wp_mobile.readerThemes
import kotlin.math.roundToInt

/** Opens `url` in another app. Returns what went wrong, when no app can open it. */
internal fun UriHandler.open(url: String): String? = try {
    openUri(url)
    null
} catch (exception: IllegalArgumentException) {
    exception.message ?: "Couldn't open $url"
}

/*
 * Shows posts from the WordPress.com Reader the way the apps will: cleaned up and put in a page by
 * `wp_mobile`'s shared Reader rendering, in a web view. It only needs public sites, so it works
 * without signing in.
 */

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReaderScreen(onSiteSubmitted: (String) -> Unit, onBackClicked: () -> Unit) {
    var site by rememberSaveable { mutableStateOf("en.blog.wordpress.com") }
    val submit = { readerSite(site).takeIf { it.isNotEmpty() }?.let(onSiteSubmitted) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Reader") },
                navigationIcon = { BackButton(onBackClicked) }
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier.fillMaxSize().padding(paddingValues).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            OutlinedTextField(
                value = site,
                onValueChange = { site = it },
                label = { Text("Site address or ID") },
                supportingText = { Text("A public site, like en.blog.wordpress.com, from the Reader API.") },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go),
                keyboardActions = KeyboardActions(onGo = { submit() }),
                modifier = Modifier.fillMaxWidth()
            )
            Button(onClick = { submit() }, enabled = readerSite(site).isNotEmpty()) {
                Text("Show Posts")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReaderPostListScreen(site: String, onPostClicked: (ULong) -> Unit, onBackClicked: () -> Unit) {
    var posts by remember { mutableStateOf<List<ReaderApi.PostSummary>?>(null) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(site) {
        try {
            posts = ReaderApi.posts(site)
        } catch (exception: ReaderApiException) {
            error = exception.message
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(site) },
                navigationIcon = { BackButton(onBackClicked) }
            )
        }
    ) { paddingValues ->
        val modifier = Modifier.fillMaxSize().padding(paddingValues)
        val loaded = posts
        when {
            loaded != null -> LazyColumn(modifier = modifier) {
                items(loaded, key = { it.id.toLong() }) { post ->
                    ListItem(
                        headlineContent = { Text(post.title, maxLines = 2, overflow = TextOverflow.Ellipsis) },
                        supportingContent = { Text(post.date) },
                        modifier = Modifier.clickable { onPostClicked(post.id) }
                    )
                    HorizontalDivider()
                }
            }
            error != null -> ErrorMessage(error.orEmpty(), modifier)
            else -> LoadingIndicator(modifier)
        }
    }
}

/** The reader's settings, which a real app saves. */
private data class ReaderSettings(
    val theme: ReaderTheme = ReaderTheme.SYSTEM,
    val font: ReaderFont = ReaderFont.SANS,
    val textScale: Double = 1.0
)

private val FONTS = listOf(ReaderFont.SANS to "Sans", ReaderFont.SERIF to "Serif", ReaderFont.MONO to "Mono")

private val TEXT_SCALES = listOf(0.75, 0.9, 1.0, 1.15, 1.25, 1.4, 1.6)

/** The web view lays the post out with margins, and draws the theme's background around it. */
private const val APP_CSS = "html { background-color: var(--reader-background); } body { padding: 16px; }"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ReaderPostScreen(
    site: String,
    postId: ULong,
    onPostClicked: (site: String, postId: ULong) -> Unit,
    onBackClicked: () -> Unit
) {
    var post by remember { mutableStateOf<ReaderApi.Post?>(null) }
    var content by remember { mutableStateOf<ReaderPostContent?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var settings by remember { mutableStateOf(ReaderSettings()) }
    var viewedImage by remember { mutableStateOf<String?>(null) }
    val uriHandler = LocalUriHandler.current
    val appColors = readerAppColors()
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    LaunchedEffect(site, postId) {
        try {
            val loaded = ReaderApi.post(site, postId)
            val cleaned = ReaderPostContent(loaded.content, loaded.url)
            // Photon, which makes the smaller image sizes, can't fetch a private site's images.
            content = if (loaded.siteIsPrivate) cleaned else cleaned.withPhotonSizes()
            post = loaded
        } catch (exception: ReaderApiException) {
            error = exception.message
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(post?.title.orEmpty(), maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = { BackButton(onBackClicked) },
                actions = { ReaderSettingsMenu(settings, onChange = { settings = it }) }
            )
        },
        snackbarHost = { SnackbarHost(snackbarHostState) }
    ) { paddingValues ->
        val modifier = Modifier.fillMaxSize().padding(paddingValues)
        val loadedPost = post
        val loadedContent = content
        when {
            loadedPost != null && loadedContent != null -> {
                val html = remember(loadedContent, settings, appColors) {
                    loadedContent.pageHtml(
                        ReaderPageOptions(
                            theme = settings.theme,
                            appColors = appColors,
                            font = settings.font,
                            textScale = settings.textScale,
                            css = APP_CSS
                        )
                    )
                }
                ReaderPostPage(
                    html = html,
                    postUrl = loadedPost.url,
                    onLink = { link, url ->
                        when (link) {
                            is ReaderLink.SitePost -> onPostClicked(link.siteId.toString(), link.postId)
                            is ReaderLink.Image -> viewedImage = link.url
                            // A real app opens the rest in the Reader, a player, or the image
                            // viewer. Here, the system opens them.
                            else -> uriHandler.open(url)?.let { message ->
                                scope.launch { snackbarHostState.showSnackbar(message) }
                            }
                        }
                    },
                    onImage = { viewedImage = it },
                    modifier = modifier
                )
            }
            error != null -> ErrorMessage(error.orEmpty(), modifier)
            else -> LoadingIndicator(modifier)
        }
    }

    viewedImage?.let { url ->
        ReaderImageViewer(url, onDismiss = { viewedImage = null })
    }
}

@Composable
private fun ReaderSettingsMenu(settings: ReaderSettings, onChange: (ReaderSettings) -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    IconButton(onClick = { expanded = true }) {
        Icon(Icons.Default.FormatSize, contentDescription = "Reading settings")
    }
    val choose = { changed: ReaderSettings ->
        expanded = false
        onChange(changed)
    }
    DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
        MenuHeading("Theme")
        readerThemes().forEach { theme ->
            MenuChoice(theme.toString(), selected = theme == settings.theme) {
                choose(settings.copy(theme = theme))
            }
        }
        HorizontalDivider()
        MenuHeading("Font")
        FONTS.forEach { (font, name) ->
            MenuChoice(name, selected = font == settings.font) { choose(settings.copy(font = font)) }
        }
        HorizontalDivider()
        MenuHeading("Text size")
        TEXT_SCALES.forEach { scale ->
            MenuChoice("${(scale * 100).roundToInt()}%", selected = scale == settings.textScale) {
                choose(settings.copy(textScale = scale))
            }
        }
    }
}

@Composable
private fun MenuHeading(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.Bold,
        modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp)
    )
}

@Composable
private fun MenuChoice(text: String, selected: Boolean, onClick: () -> Unit) {
    DropdownMenuItem(
        text = { Text(if (selected) "✓ $text" else text) },
        onClick = onClick
    )
}

@Composable
private fun BackButton(onClick: () -> Unit) {
    IconButton(onClick = onClick) {
        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
    }
}

/**
 * The app's own colours, for the System theme: Material's, with the WordPress app's colour for links.
 */
@Composable
private fun readerAppColors(): ReaderAppColors = remember {
    val defaults = readerDefaultAppColors(ReaderBrand.WORDPRESS)
    ReaderAppColors(
        light = lightColorScheme().readerColors(defaults.light),
        dark = darkColorScheme().readerColors(defaults.dark)
    )
}

private fun ColorScheme.readerColors(defaults: ReaderThemeColors): ReaderThemeColors {
    fun color(color: Color) = readerColorFromArgb(color.toArgb())
    return ReaderThemeColors(
        text = color(onSurface),
        secondaryText = color(onSurfaceVariant),
        tertiaryText = color(outline),
        background = color(surface),
        secondaryBackground = color(surfaceVariant),
        border = color(outlineVariant),
        link = defaults.link
    )
}

/**
 * The post's page, in the platform's web view. It handles the shared script's messages, and the
 * post's links: anchors itself, and the rest with [onLink], with the link's address. [onImage] is
 * for images the reader taps, which open in the image viewer.
 */
@Composable
expect fun ReaderPostPage(
    html: String,
    postUrl: String?,
    onLink: (link: ReaderLink, url: String) -> Unit,
    onImage: (url: String) -> Unit,
    modifier: Modifier = Modifier
)

@Composable
private fun ReaderImageViewer(url: String, onDismiss: () -> Unit) {
    Dialog(onDismissRequest = onDismiss) {
        Surface(shape = MaterialTheme.shapes.large) {
            Column(horizontalAlignment = Alignment.End) {
                CoilImage(
                    imageModel = { url },
                    modifier = Modifier.fillMaxWidth()
                )
                TextButton(onClick = onDismiss, modifier = Modifier.padding(8.dp)) { Text("Done") }
            }
        }
    }
}
