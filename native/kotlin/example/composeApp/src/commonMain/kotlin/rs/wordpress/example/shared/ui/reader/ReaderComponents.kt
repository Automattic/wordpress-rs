package rs.wordpress.example.shared.ui.reader

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex

/**
 * A Reader screen's frame: its title, the way back, a spinner while a request runs, and under the
 * title, where it stays in view however far the screen is scrolled, what the last request said.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun ReaderScaffold(
    title: String,
    requests: ReaderRequests,
    onBackClicked: () -> Unit,
    content: @Composable () -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title, maxLines = 1) },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    if (requests.isRunning) {
                        CircularProgressIndicator(
                            modifier = Modifier.padding(end = 16.dp).size(20.dp).testTag("reader_busy"),
                            strokeWidth = 2.dp
                        )
                    }
                }
            )
        }
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            ReaderStatus(requests)
            content()
        }
    }
}

/**
 * For a line that stays in view above a list. A list's row that's scrolled partly out of view
 * still reaches up over the line as far as accessibility tools can tell, and a line they take to
 * be covered is left out of what they read. Drawn above the list, it isn't.
 */
private fun Modifier.pinnedAboveList(): Modifier = fillMaxWidth().zIndex(1f)

/** What the screen's last request said: an action's outcome, or an error, in words. */
@Composable
private fun ReaderStatus(requests: ReaderRequests) {
    val message = requests.message ?: return
    Text(
        text = if (requests.isError) "Error: $message" else message,
        color = if (requests.isError) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary,
        style = MaterialTheme.typography.bodyMedium,
        modifier = Modifier.pinnedAboveList().padding(horizontal = 16.dp, vertical = 8.dp).testTag("reader_status")
    )
}

@Composable
internal fun ReaderSectionTitle(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 20.dp, bottom = 4.dp)
    )
}

/** A row that opens something. */
@Composable
internal fun ReaderLinkRow(title: String, subtitle: String? = null, onClicked: () -> Unit) {
    ListItem(
        headlineContent = { Text(title) },
        supportingContent = subtitle?.let { { Text(it) } },
        trailingContent = {
            Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
        },
        modifier = Modifier.clickable(onClick = onClicked)
    )
}

/** A row of a field and the buttons that use it, or of buttons alone. */
@Composable
internal fun ReaderControlsRow(content: @Composable RowScope.() -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        content = content
    )
}

/** A line of small print. [tag] is what a UI test finds it by. */
@Composable
internal fun ReaderNote(text: String, tag: String? = null) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = (if (tag != null) Modifier.pinnedAboveList().testTag(tag) else Modifier.fillMaxWidth())
            .padding(horizontal = 16.dp, vertical = 4.dp)
    )
}

/** A line of small print that says what went wrong. */
@Composable
internal fun ReaderErrorNote(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.error,
        modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 4.dp).testTag("reader_error")
    )
}
