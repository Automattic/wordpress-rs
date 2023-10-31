pluginManagement {
    plugins {
        id("com.android.library") version "8.1.0"
        id("org.jetbrains.kotlin.android") version "1.8.20"
        id("org.mozilla.rust-android-gradle.rust-android") version "0.9.3"
    }
    repositories {
        gradlePluginPortal()
        google()
    }
}
plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "0.7.0"
}

rootProject.name = "wordpress-rs"
include("lib")
