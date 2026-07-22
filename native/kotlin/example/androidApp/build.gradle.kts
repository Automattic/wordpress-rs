import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinJvmCompile

plugins {
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.jetbrainsCompose)
    alias(libs.plugins.compose.compiler)
}

tasks.withType<KotlinJvmCompile>().configureEach {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_21)
        freeCompilerArgs.add("-opt-in=kotlin.RequiresOptIn")
    }
}

android {
    namespace = "rs.wordpress.example"
    compileSdk = libs.versions.android.compileSdk.get().toInt()

    defaultConfig {
        applicationId = "rs.wordpress.example"
        minSdk = libs.versions.android.minSdk.get().toInt()
        targetSdk = libs.versions.android.targetSdk.get().toInt()
        versionCode = 1
        versionName = "1.0"
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
            excludes += "/META-INF/versions/9/OSGI-INF/MANIFEST.MF"
        }
    }
    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation(project(":example:composeApp"))
    // The self-hosted API client (`:api:android`) transitively exposes `:api:kotlin` with the
    // Android JNA aar substituted for the desktop jar, so it is the only WordPress API dependency
    // this module declares.
    if (project.hasProperty("wpApiAndroidVersion")) {
        implementation("rs.wordpress.api:android:${project.properties["wpApiAndroidVersion"]}")
    } else {
        implementation(project(":api:android"))
    }
    implementation(libs.androidx.activity.compose)
    implementation(libs.koin.android)
    // The WordPress API client constructors expose `okhttp3.Interceptor` in their signatures, so
    // callers such as `WelcomeActivity` need okhttp on the compile classpath directly.
    implementation(libs.okhttp)
    implementation(compose.preview)
    debugImplementation(compose.uiTooling)
}
