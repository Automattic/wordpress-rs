import org.jetbrains.compose.desktop.application.dsl.TargetFormat

plugins {
    alias(libs.plugins.kotlinMultiplatform)
    alias(libs.plugins.androidApplication)
    alias(libs.plugins.jetbrainsCompose)
    // TODO (Kotlin-2.0) - re-enable
    // alias(libs.plugins.compose.compiler)
}

kotlin {
    androidTarget {
        // TODO (Kotlin-2.0) - re-enable
        // @OptIn(ExperimentalKotlinGradlePluginApi::class)
        // compilerOptions {
        //     jvmTarget.set(JvmTarget.JVM_11)
        // }
    }

    jvm("desktop")

    sourceSets {
        val desktopMain by getting {
            resources.srcDirs(
                listOf(
                    rootProject.ext.get("jniLibsPath"),
                    rootProject.ext.get("generatedTestResourcesPath")
                )
            )
        }

        androidMain.dependencies {
            implementation(compose.preview)
            implementation(libs.androidx.material)
            implementation(libs.androidx.activity.compose)
            implementation(libs.koin.android)
            implementation(libs.lifecycle.viewmodel.compose)
            implementation(libs.navigation.compose)
            implementation(libs.navigation.fragment.ktx)
            implementation(libs.navigation.ui.ktx)
            implementation(project(":api:android"))
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(compose.components.uiToolingPreview)
            implementation(libs.jetbrains.navigation.compose)
            implementation(libs.koin.core)
            implementation(libs.koin.compose)
            implementation(libs.kotlinx.coroutines.core)
            implementation(libs.landscapist.coil)
            implementation(libs.lifecycle.viewmodel)
            compileOnly(project(":api:kotlin"))
        }
        desktopMain.dependencies {
            implementation(compose.desktop.currentOs)
            implementation(project(":api:kotlin"))
        }
    }
}

android {
    namespace = "rs.wordpress.example"
    compileSdk = libs.versions.android.compileSdk.get().toInt()

    sourceSets["main"].manifest.srcFile("src/androidMain/AndroidManifest.xml")
    sourceSets["main"].res.srcDirs("src/androidMain/res")
    sourceSets["main"].resources.srcDirs("src/commonMain/resources")

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
        }
    }
    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        // TODO (Kotlin-2.0) - Revert back to VERSION_11
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
    }
    dependencies {
        debugImplementation(compose.uiTooling)
    }
    // TODO (Kotlin-2.0) - Remove `composeOptions`
    composeOptions {
        // Once Kotlin is upgraded to >=2.0, this should be replaced with compose compiler plugin
        // https://developer.android.com/develop/ui/compose/compiler
        kotlinCompilerExtensionVersion = "1.5.14"
    }
}

compose.desktop {
    application {
        mainClass = "rs.wordpress.example.MainKt"

        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
            packageName = "rs.wordpress.example"
            packageVersion = "1.0.0"
        }
    }
}

tasks.named("desktopProcessResources").configure {
    dependsOn(rootProject.tasks.named("copyDesktopJniLibs"))
    dependsOn(rootProject.tasks.named("copyTestCredentials"))
}
