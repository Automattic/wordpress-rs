package rs.wordpress.example.shared.ui.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@OptIn(ExperimentalMaterial3Api::class)
@Composable
@org.jetbrains.compose.ui.tooling.preview.Preview
fun LoginScreen(authenticateSite: (String) -> Unit, authenticateWpCom: (() -> Unit)? = null, onBackClicked: () -> Unit = {}) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Add Site") },
                navigationIcon = {
                    IconButton(onClick = onBackClicked) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize().padding(paddingValues),
        ) {
            var siteUrl by remember { mutableStateOf("") }
            OutlinedTextField(
                value = siteUrl,
                onValueChange = { siteUrl = it },
                label = { Text("Site URL") }
            )
            Spacer(modifier = Modifier.height(16.dp))
            Button(onClick = { authenticateSite(siteUrl) }) {
                Text("Login")
            }
            Spacer(modifier = Modifier.height(32.dp))
            HorizontalDivider(modifier = Modifier.padding(horizontal = 32.dp))
            Spacer(modifier = Modifier.height(32.dp))
            OutlinedButton(
                onClick = { authenticateWpCom?.invoke() },
                enabled = authenticateWpCom != null
            ) {
                Text("Sign in to WordPress.com")
            }
            if (authenticateWpCom == null) {
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Add client_id and client_secret to wp_com_test_credentials.json to enable",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                    modifier = Modifier.padding(horizontal = 32.dp)
                )
            }
        }
    }
}
