import org.jetbrains.compose.desktop.application.dsl.TargetFormat
import org.jetbrains.kotlin.gradle.ExperimentalKotlinGradlePluginApi
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinJvmCompile

plugins {
    alias(libs.plugins.kotlinMultiplatform)
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

// Desktop resources path - separate from integration test resources to avoid IDE conflicts
val desktopResourcesPath = layout.buildDirectory.dir("desktopResources")

// Copy resources needed for desktop app
val copyDesktopAppResources = tasks.register<Copy>("copyDesktopAppResources") {
    dependsOn(rootProject.tasks.named("copyDesktopJniLibs"))
    dependsOn(rootProject.tasks.named("copyTestCredentials"))
    dependsOn(rootProject.tasks.named("copyTestMedia"))
    dependsOn(rootProject.tasks.named("copySampleJSON"))
    from(rootProject.ext.get("jniLibsPath"))
    from(rootProject.ext.get("generatedTestResourcesPath"))
    into(desktopResourcesPath)
}

kotlin {
    androidTarget()
    jvm("desktop")

    sourceSets {
        val desktopMain by getting {
            resources.srcDirs(desktopResourcesPath)
        }

        androidMain.dependencies {
            implementation(compose.preview)
            implementation(libs.androidx.activity.compose)
            implementation(libs.koin.android)
            implementation(libs.lifecycle.viewmodel.compose)
            implementation(libs.navigation.compose)
            implementation(libs.navigation.fragment.ktx)
            implementation(libs.navigation.ui.ktx)
            if (project.hasProperty("wpApiAndroidVersion")) {
                implementation("rs.wordpress.api:android:${project.properties["wpApiAndroidVersion"]}")
            } else {
                implementation(project(":api:android"))
            }
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.material3)
            implementation(compose.materialIconsExtended)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(compose.components.uiToolingPreview)
            implementation(libs.jetbrains.navigation.compose)
            implementation(libs.koin.core)
            implementation(libs.koin.compose)
            implementation(libs.kotlinx.coroutines.core)
            implementation(libs.landscapist.coil)
            implementation(libs.lifecycle.viewmodel)
            if (project.hasProperty("wpApiKotlinVersion")) {
                compileOnly("rs.wordpress.api:kotlin:${project.properties["wpApiKotlinVersion"]}")
            } else {
                compileOnly(project(":api:kotlin"))
            }
        }
        desktopMain.dependencies {
            implementation(compose.desktop.currentOs)
            if (project.hasProperty("wpApiKotlinVersion")) {
                implementation("rs.wordpress.api:kotlin:${project.properties["wpApiKotlinVersion"]}")
            } else {
                implementation(project(":api:kotlin"))
            }
        }
    }
}

android {
    namespace = "rs.wordpress.example"
    compileSdk = libs.versions.android.compileSdk.get().toInt()

    sourceSets["main"].manifest.srcFile("src/androidMain/AndroidManifest.xml")
    sourceSets["main"].res.srcDirs("src/androidMain/res")

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
    dependencies {
        debugImplementation(compose.uiTooling)
    }
}

compose.desktop {
    application {
        mainClass = "rs.wordpress.example.MainKt"

        jvmArgs += listOf(
            "-Djna.library.path=.",
            "-Djava.library.path=."
        )

        nativeDistributions {
            targetFormats(TargetFormat.Dmg, TargetFormat.Msi, TargetFormat.Deb)
            packageName = "rs.wordpress.example"
            packageVersion = "1.0.0"
        }
    }
}

// Generate BuildConfig with rust module name
val generateBuildConfig = tasks.register("generateBuildConfig") {
    val outputDir = layout.buildDirectory.dir("generated/source/buildConfig")
    val rustPrimaryModule = rootProject.ext.get("rustPrimaryModule") as String

    outputs.dir(outputDir)

    doLast {
        val buildConfigFile = outputDir.get().file("rs/wordpress/example/BuildConfig.kt").asFile
        buildConfigFile.parentFile.mkdirs()
        buildConfigFile.writeText("""
            package rs.wordpress.example

            object BuildConfig {
                const val RUST_PRIMARY_MODULE = "$rustPrimaryModule"
            }
        """.trimIndent())
    }
}

// Generate TestCredentials from test_credentials.json
val generateTestCredentials = tasks.register("generateTestCredentials") {
    val outputDir = layout.buildDirectory.dir("generated/source/testCredentials")
    val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot") as String
    val credentialsFile = file("$cargoProjectRoot/test_credentials.json")

    // Only mark as input if file exists - allows build to work without test server running
    inputs.files(credentialsFile).optional(true)
    outputs.dir(outputDir)

    doLast {
        val testCredentialsFile = outputDir.get().file("rs/wordpress/example/TestCredentials.kt").asFile
        testCredentialsFile.parentFile.mkdirs()

        if (credentialsFile.exists()) {
            val json = groovy.json.JsonSlurper().parseText(credentialsFile.readText()) as Map<*, *>
            testCredentialsFile.writeText("""
                package rs.wordpress.example

                object TestCredentials {
                    val SITE_URL: String? = "${json["site_url"]}"
                    val ADMIN_USERNAME: String? = "${json["admin_username"]}"
                    val ADMIN_PASSWORD: String? = "${json["admin_password"]}"
                    val SUBSCRIBER_USERNAME: String? = "${json["subscriber_username"]}"
                    val SUBSCRIBER_PASSWORD: String? = "${json["subscriber_password"]}"
                }
            """.trimIndent())
        } else {
            testCredentialsFile.writeText("""
                package rs.wordpress.example

                object TestCredentials {
                    val SITE_URL: String? = null
                    val ADMIN_USERNAME: String? = null
                    val ADMIN_PASSWORD: String? = null
                    val SUBSCRIBER_USERNAME: String? = null
                    val SUBSCRIBER_PASSWORD: String? = null
                }
            """.trimIndent())
        }
    }
}

kotlin.sourceSets.getByName("desktopMain") {
    kotlin.srcDir(layout.buildDirectory.dir("generated/source/buildConfig"))
}

kotlin.sourceSets.getByName("commonMain") {
    kotlin.srcDir(layout.buildDirectory.dir("generated/source/testCredentials"))
    kotlin.srcDir(layout.buildDirectory.dir("generated/source/wpComCredentials"))
}

// Generate WpComCredentials from wp_com_test_credentials.json
val generateWpComCredentials = tasks.register("generateWpComCredentials") {
    val outputDir = layout.buildDirectory.dir("generated/source/wpComCredentials")
    val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot") as String
    val credentialsFile = file("$cargoProjectRoot/wp_com_test_credentials.json")

    inputs.files(credentialsFile).optional(true)
    outputs.dir(outputDir)

    doLast {
        val wpComCredentialsFile = outputDir.get().file("rs/wordpress/example/WpComCredentials.kt").asFile
        wpComCredentialsFile.parentFile.mkdirs()

        if (credentialsFile.exists()) {
            val json = groovy.json.JsonSlurper().parseText(credentialsFile.readText()) as Map<*, *>
            val clientId = json["client_id"]
            val clientSecret = json["client_secret"]
            wpComCredentialsFile.writeText("""
                package rs.wordpress.example

                object WpComCredentials {
                    val CLIENT_ID: ULong? = ${if (clientId != null) "${clientId}uL" else "null"}
                    val CLIENT_SECRET: String? = ${if (clientSecret != null) "\"$clientSecret\"" else "null"}
                }
            """.trimIndent())
        } else {
            wpComCredentialsFile.writeText("""
                package rs.wordpress.example

                object WpComCredentials {
                    val CLIENT_ID: ULong? = null
                    val CLIENT_SECRET: String? = null
                }
            """.trimIndent())
        }
    }
}

tasks.named("compileKotlinDesktop").configure {
    dependsOn(generateBuildConfig)
    dependsOn(generateTestCredentials)
    dependsOn(generateWpComCredentials)
}

// Ensure generated sources are available before any Android compilation
tasks.named("preBuild").configure {
    dependsOn(generateTestCredentials)
    dependsOn(generateWpComCredentials)
}

tasks.named("desktopProcessResources").configure {
    dependsOn(copyDesktopAppResources)
}
