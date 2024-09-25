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

    // There is an incorrect lint error in generated jetpack_api.kt about the usage of NewApi
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
val moduleName = "jetpack_api"
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