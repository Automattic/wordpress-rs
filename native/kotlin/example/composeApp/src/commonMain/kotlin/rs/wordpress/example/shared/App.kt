package rs.wordpress.example.shared

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import org.jetbrains.compose.ui.tooling.preview.Preview
import org.koin.compose.koinInject
import rs.wordpress.example.shared.ui.users.UserListViewModel
import uniffi.wp_api.UserWithEditContext

@Composable
@Preview
fun App(userListViewModel: UserListViewModel = koinInject()) {
    MaterialTheme {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize(),
        ) {
            var showContent by remember { mutableStateOf(false) }
            if (!showContent) {
                Button(onClick = { showContent = !showContent }) {
                    Text("Fetch Users")
                }
            }
            AnimatedVisibility(showContent) {
                LazyColumn {
                    items(userListViewModel.fetchUsers()) {
                        UserCard(it)
                    }
                }
            }
        }
    }
}

@Composable
fun UserCard(user: UserWithEditContext) {
    Row(modifier = Modifier.padding(all = 8.dp)) {
        Column {
            Text(
                text = user.name,
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text =  user.email,
            )
        }
    }
}
