plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("com.automattic.android.publish-to-s3")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

android {
    namespace = "rs.wordpress.api.android"

    compileSdk = libs.versions.android.compileSdk.get().toInt()

    // `ndkVersion` will be set to the version defined by Android Gradle Plugin, but it still needs
    // to be manually installed: https://developer.android.com/build/releases/gradle-plugin#compatibility
    // Note that if the project's AGP version is not up to date, we need to find the correct release
    // notes from the list: https://developer.android.com/build/releases/past-releases (on the left side)
    //
    // TODO: The comment above is temporarily incorrect, as we are using a specific NDK version to
    // test the 16kb page size changes. When we update AGP to a version that is packaged with NDK 
    // version above `28`, we should remove this.
    ndkVersion = "28.1.13356709"

    defaultConfig {
        minSdk = libs.versions.android.minSdk.get().toInt()

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildFeatures {
        buildConfig = true
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

    // The Rust `.so` files produced by the `cargoBuild` task (below) are written here; registering
    // the directory as a jniLibs source set makes AGP package them into the AAR and the test APK.
    sourceSets["main"].jniLibs.srcDir("${layout.buildDirectory.get()}/rustJniLibs/android")
    sourceSets["androidTest"].jniLibs.srcDir("${layout.buildDirectory.get()}/rustJniLibs/android")
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
    implementation(libs.okhttp.tls)
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

// --- Rust native library build ---
//
// Cross-compiles the Rust crate for each Android ABI with `cargo ndk`, writing the resulting `.so`
// files to `build/rustJniLibs/android/<abi>/` — registered as a jniLibs source directory above so
// AGP packages them into the AAR.
//
// Requires the `cargo-ndk` cargo subcommand (`cargo install cargo-ndk`) and the NDK named by
// `ndkVersion` above to be installed.
val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot").toString()
val rustModule = rootProject.ext.get("rustPrimaryModule").toString()
val rustJniLibsDir = layout.buildDirectory.dir("rustJniLibs/android")

// Map the `-PcargoTarget` name passed for single-ABI builds (e.g. `arm64`) to the Android ABI
// understood by `cargo ndk`. With no property set, all four ABIs are built.
val gradleTargetToAbi = mapOf(
    "arm" to "armeabi-v7a",
    "arm64" to "arm64-v8a",
    "x86" to "x86",
    "x86_64" to "x86_64",
)
val cargoAbis: List<String> = project.findProperty("cargoTarget")?.let { property ->
    val target = property.toString()
    listOf(gradleTargetToAbi[target] ?: error("Unknown cargoTarget '$target'"))
} ?: gradleTargetToAbi.values.toList()

tasks.register<Exec>("cargoBuild") {
    group = "rust"
    description = "Cross-compiles the Rust native library for Android via cargo-ndk"

    workingDir(cargoProjectRoot)

    // Point cargo-ndk at the same NDK that AGP is configured to use.
    environment("ANDROID_NDK_HOME", android.sdkDirectory.resolve("ndk/${android.ndkVersion}").absolutePath)
    // Include debug information in the release build.
    environment("RUSTFLAGS", "-g")

    commandLine(
        buildList {
            add(rootProject.ext.get("cargoBinaryPath").toString())
            add("ndk")
            add("--output-dir"); add(rustJniLibsDir.get().asFile.absolutePath)
            add("--platform"); add(libs.versions.android.minSdk.get())
            cargoAbis.forEach { abi -> add("-t"); add(abi) }
            add("build"); add("--release"); add("--package"); add(rustModule)
        }
    )

    outputs.dir(rustJniLibsDir)

    doLast {
        // cargo-ndk copies every cdylib the build produces; keep only the primary library. `wp_api`
        // also declares a `cdylib` crate type, but its code is statically linked into
        // `libwp_mobile.so` — the single library the generated UniFFI bindings load — so its
        // standalone `.so` is redundant weight in the AAR.
        val keep = "lib$rustModule.so"
        rustJniLibsDir.get().asFile.walkTopDown()
            .filter { it.isFile && it.extension == "so" && it.name != keep }
            .forEach { it.delete() }
    }
}

tasks.matching { it.name.matches("merge.*JniLibFolders".toRegex()) }.configureEach {
    inputs.dir(rustJniLibsDir)
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
