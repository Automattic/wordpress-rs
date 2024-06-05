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

    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        debug {
            readTestCredentials()?.let {
                buildConfigField("String", "TEST_SITE_URL", "\"${it.siteUrl}\"")
                buildConfigField("String", "TEST_ADMIN_USERNAME", "\"${it.adminUsername}\"")
                buildConfigField("String", "TEST_ADMIN_PASSWORD", "\"${it.adminPassword}\"")
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

    sourceSets["androidTest"].jniLibs.srcDirs.plus("${layout.buildDirectory.get()}/rustJniLibs/android")
}

repositories {
    maven {
        url = uri("https://a8c-libs.s3.amazonaws.com/android")
        content {
            includeGroup("rs.wordpress.api")
        }
    }
    mavenCentral()
    google()
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
    targets = listOf("arm", "arm64", "x86", "x86_64", nativeRustTarget())
    targetDirectory = "$cargoProjectRoot/target"
    exec = { spec: ExecSpec, _: com.nishtahir.Toolchain ->
        // https://doc.rust-lang.org/rustc/command-line-arguments.html#-g-include-debug-information
        spec.environment("RUSTFLAGS", "-g")
    }
}
tasks.matching { it.name.matches("merge.*JniLibFolders".toRegex()) }.configureEach {
    inputs.dir(File("${layout.buildDirectory.get()}/rustJniLibs/android"))
    dependsOn("cargoBuild")
}

tasks.matching { it.name.matches("test".toRegex()) }.configureEach {
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

fun nativeRustTarget(): String {
    return when (val resourcePrefix = com.sun.jna.Platform.RESOURCE_PREFIX) {
        // For unit tests to work in Apple Silicon, we need to return "darwin-aarch64" here
        // However, that runs the cargo task as `cargoBuildDarwin-aarch64` which is not properly
        // cached by cargo and requires a rebuild every time. This results in a significant
        // development time loss, so for now, we are returning "darwin" and using instrumented
        // tests instead.
        "darwin" -> "darwin"
        "darwin-aarch64" -> "darwin-aarch64"
        "darwin-x86-64" -> "darwin-x86-64"
        "linux-x86-64" -> "linux-x86-64"
        "win32-x86-64" -> "win32-x86-64-gnu"
        else -> throw GradleException("Unsupported Platform: $resourcePrefix")
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
        subscriberUsername = lines[3],
        subscriberPassword = lines[4]
    )
}

data class TestCredentials(
    val siteUrl: String,
    val adminUsername: String,
    val adminPassword: String,
    val subscriberUsername: String,
    val subscriberPassword: String,
)
