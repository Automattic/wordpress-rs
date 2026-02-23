package rs.wordpress.example.shared.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import rs.wordpress.example.shared.ui.stresstest.PostDisplayData

@Composable
fun PostCard(post: PostDisplayData) {
    ElevatedCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Row(modifier = Modifier.padding(12.dp)) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = post.title,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = post.contentPreview,
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2
                )
                Spacer(modifier = Modifier.height(4.dp))
                Row {
                    Text(
                        text = "Status: ${post.status}",
                        style = MaterialTheme.typography.bodySmall
                    )
                    if (post.author != null) {
                        Text(
                            text = " • Author: ${post.author}",
                            style = MaterialTheme.typography.bodySmall
                        )
                    }
                }
                Text(
                    text = "Modified: ${post.modified}",
                    style = MaterialTheme.typography.bodySmall
                )
            }
        }
    }
}
