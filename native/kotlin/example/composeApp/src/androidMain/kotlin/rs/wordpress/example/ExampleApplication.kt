package rs.wordpress.example

import android.app.Application
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.util.Log
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import org.koin.dsl.module
import rs.wordpress.api.android.KeystorePasswordTransformer
import rs.wordpress.api.kotlin.NetworkAvailabilityProvider
import rs.wordpress.example.shared.di.commonModules
import uniffi.wp_mobile.AccountRepository
import uniffi.wp_mobile.AccountRepositoryException

class ExampleApplication: Application() {
    override fun onCreate() {
        super.onCreate()

        val networkModule = module {
            single {
                NetworkAvailabilityProvider {
                    val cm = getSystemService(ConnectivityManager::class.java)
                    val capabilities = cm.getNetworkCapabilities(cm.activeNetwork)
                    capabilities?.hasCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET) == true
                }
            }
        }

        val transformer = KeystorePasswordTransformer("rs.wordpress.example")
        val rootPath = filesDir.resolve("accounts").absolutePath

        val accountRepository = try {
            val repo = AccountRepository(rootPath, transformer)
            repo.all()
            repo
        } catch (e: AccountRepositoryException.PasswordException) {
            // Existing data was encrypted with a different key (e.g., after
            // reinstall). Discard the unrecoverable data and start fresh.
            Log.w("ExampleApplication", "Clearing unreadable account data", e)
            filesDir.resolve("accounts").deleteRecursively()
            AccountRepository(rootPath, transformer)
        }

        startKoin {
            androidContext(this@ExampleApplication)
            modules(listOf(networkModule) + commonModules(accountRepository))
        }
    }
}
