package rs.wordpress.example

import android.app.Application
import org.koin.android.ext.koin.androidContext
import org.koin.core.context.startKoin
import rs.wordpress.example.shared.di.commonModule

class ExampleApplication: Application() {
    override fun onCreate() {
        super.onCreate()

        startKoin {
            androidContext(this@ExampleApplication)
            modules(commonModule())
        }
    }
}