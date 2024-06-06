package rs.wordpress.example

import android.app.Application
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import org.koin.dsl.module
import rs.wordpress.example.shared.di.commonModules
import rs.wordpress.example.ui.login.LoginViewModel

class ExampleApplication: Application() {
    override fun onCreate() {
        super.onCreate()

        startKoin {
            androidContext(this@ExampleApplication)
            modules(commonModules().plus(androidOnlyModules))
        }
    }
}

private val androidOnlyModules = module {
    single { LoginViewModel(get()) }
}