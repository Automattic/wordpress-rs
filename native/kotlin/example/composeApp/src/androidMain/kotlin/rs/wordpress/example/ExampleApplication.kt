package rs.wordpress.example

import android.app.Application
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import rs.wordpress.api.android.KeystorePasswordTransformer
import rs.wordpress.example.shared.di.commonModules
import uniffi.wp_mobile.AccountRepository

class ExampleApplication: Application() {
    override fun onCreate() {
        super.onCreate()

        val accountRepository = AccountRepository(
            rootPath = filesDir.resolve("accounts").absolutePath,
            passwordTransformer = KeystorePasswordTransformer()
        )

        startKoin {
            androidContext(this@ExampleApplication)
            modules(commonModules(accountRepository))
        }
    }
}
