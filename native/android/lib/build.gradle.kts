plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.mozilla.rust-android-gradle.rust-android")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

android {
    namespace = "org.wordpress.rs"

    compileSdk = 33

    defaultConfig {
        minSdk = 24

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

}

repositories {
    mavenCentral()
    google()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.7.0")

    testImplementation("junit:junit:4.13.2")
}

cargo {
    module  = "../../../wordpress_api/"
    libname = "wordpress_api"
    targets = listOf("arm", "arm64", "x86")
}
