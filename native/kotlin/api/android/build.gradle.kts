plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("org.mozilla.rust-android-gradle.rust-android")
    id("com.automattic.android.publish-to-s3")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

android {
    namespace = "rs.wordpress.api.android"

    compileSdk = libs.versions.android.compileSdk.get().toInt()

    // `ndkVersion` will be set to the version defined by Android Gradle Plugin, but it still needs
    // to be manually installed: https://developer.android.com/build/releases/gradle-plugin#compatibility
    // Note that if the project's AGP version is not up to date, we need to find the correct release
    // notes from the list: https://developer.android.com/build/releases/past-releases (on the left side)

    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        debug {
            // TODO: Test credentials shouldn't be included while publishing
            readTestCredentials()?.let {
                buildConfigField("String", "TEST_SITE_URL", "\"${it.siteUrl}\"")
                buildConfigField("String", "TEST_ADMIN_USERNAME", "\"${it.adminUsername}\"")
                buildConfigField("String", "TEST_ADMIN_PASSWORD", "\"${it.adminPassword}\"")
                buildConfigField("String", "TEST_ADMIN_PASSWORD_UUID", "\"${it.adminPasswordUuid}\"")
                buildConfigField(
                    "String",
                    "TEST_SUBSCRIBER_USERNAME",
                    "\"${it.subscriberUsername}\""
                )
                buildConfigField(
                    "String",
                    "TEST_SUBSCRIBER_PASSWORD",
                    "\"${it.subscriberPassword}\""
                )
                buildConfigField("String", "TEST_SUBSCRIBER_PASSWORD_UUID", "\"${it.subscriberPasswordUuid}\"")
            }
        }
    }

    // There is an incorrect lint error in generated wp_api.kt about the usage of NewApi
    // that's related to the usage of `android.system.SystemCleaner`.
    //
    // At the time of this comment, generated bindings only use this `SystemCleaner` for
    // API's above 33 and fallback to Jna cleaner `UniffiJnaCleaner` that's available for
    // earlier APIs.
    //
    // Instead of completely ignoring this issue, we are tracking it through the baseline lint
    // file - at least for now.
    lint.baseline = file("${project.rootDir}/config/lint/baseline.xml")
}

dependencies {
    if (project.hasProperty("wpApiKotlinVersion")) {
        api("rs.wordpress.api:kotlin:${project.properties["wpApiKotlinVersion"]}") {
            exclude(group = "net.java.dev.jna")
        }
    } else {
        api(project(":api:kotlin")) {
            exclude(group = "net.java.dev.jna")
        }
    }
    implementation(libs.okhttp)
    implementation(libs.androidx.annotation)
    implementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }

    androidTestImplementation(libs.androidx.runner)
    androidTestImplementation(libs.androidx.rules)
    androidTestImplementation(libs.junit)
    androidTestImplementation(libs.kotlin.test)
    androidTestImplementation(libs.jna) {
        artifact {
            type = "aar"
        }
    }
    androidTestImplementation(libs.kotlinx.coroutines.test)

    testImplementation(libs.junit)
    testImplementation(libs.jna)
}

val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot")!!
val moduleName = "wp_api"
cargo {
    module = "$cargoProjectRoot/$moduleName/"
    libname = moduleName
    profile = "release"
    targets = listOf("arm", "arm64", "x86", "x86_64")
    targetDirectory = "$cargoProjectRoot/target"
    exec = { spec: ExecSpec, _: com.nishtahir.Toolchain ->
        // https://doc.rust-lang.org/rustc/command-line-arguments.html#-g-include-debug-information
        spec.environment("RUSTFLAGS", "-g")
    }
}
tasks.matching { it.name.matches("merge.*JniLibFolders".toRegex()) }.configureEach {
    dependsOn("cargoBuild")
}

project.afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("maven") {
                from(components["release"])

                groupId = "rs.wordpress.api"
                artifactId = "android"
                // version is set by 'publish-to-s3' plugin
            }
        }
    }
}

fun readTestCredentials(): TestCredentials? {
    val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot")
    val credentialsFile = rootProject.file("$cargoProjectRoot/test_credentials")
    if (!credentialsFile.exists()) {
        return null
    }
    val lines = credentialsFile.readLines()
    // https://developer.android.com/studio/run/emulator-networking
    val siteUrl = if (lines[0] == "http://localhost") {
        "http://10.0.2.2"
    } else {
        lines[0]
    }
    return TestCredentials(
        siteUrl = siteUrl,
        adminUsername = lines[1],
        adminPassword = lines[2],
        adminPasswordUuid = lines[3],
        subscriberUsername = lines[4],
        subscriberPassword = lines[5],
        subscriberPasswordUuid = lines[6]
    )
}

data class TestCredentials(
    val siteUrl: String,
    val adminUsername: String,
    val adminPassword: String,
    val adminPasswordUuid: String,
    val subscriberUsername: String,
    val subscriberPassword: String,
    val subscriberPasswordUuid: String,
)
