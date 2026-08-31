import java.io.File

plugins {
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.kotlinSerialization)
    alias(libs.plugins.publishToS3)
    id("java-library")
    id("jvm-test-suite")
    id("ai-docs")
}

java {
    withJavadocJar()
    withSourcesJar()

    sourceCompatibility = JavaVersion.VERSION_21
    targetCompatibility = JavaVersion.VERSION_21
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}

@Suppress("UnstableApiUsage")
testing {
    suites {
        val test by getting(JvmTestSuite::class) {
            useJUnit(rootProject.libs.versions.junit.get())

            dependencies {
                implementation(libs.okhttp)
            }
        }

        register<JvmTestSuite>("integrationTest") {
            sources {
                resources {
                    setSrcDirs(
                        listOf(
                            rootProject.ext.get("jniLibsPath"),
                            rootProject.ext.get("generatedTestResourcesPath")
                        )
                    )
                }
            }

            dependencies {
                implementation(project())

                implementation(libs.okhttp)
                implementation(libs.okhttp.mockwebserver)
                implementation(libs.okhttp.tls)
                implementation(rootProject.libs.kotlin.test)
                implementation(rootProject.libs.kotlinx.coroutines.test)
                implementation(libs.kotlinx.serialization)
            }

            targets {
                all {
                    testTask.configure {
                        shouldRunAfter(test)
                    }
                }
            }
        }
    }
}

tasks.withType<Test>().configureEach {
    afterTest(KotlinClosure2({ descriptor: TestDescriptor, result: TestResult ->
        println("[${descriptor.className}] > ${descriptor.displayName}: ${result.resultType}")
    }))
}

@Suppress("UnstableApiUsage")
tasks.named("check") {
    dependsOn(testing.suites.named("integrationTest"))
}

dependencies {
    implementation(libs.okhttp)
    implementation(libs.okhttp.tls)
    implementation(libs.jna)
    implementation(libs.kotlinx.coroutines.core)
}

sourceSets {
    main {
        java {
            srcDir("${layout.buildDirectory.get()}/generated/source/uniffi/java/uniffi")
        }
    }
}

// Post-processes a generated UniFFI Kotlin binding to give every error type that supports
// localization a `localizedDescription(locale)` extension, at parity with Swift's generated
// `LocalizedError.errorDescription` (see `scripts/swift-bindings.sh` →
// `generate_localized_error_extension`). Keyed off the generated `localize<Type>(value: T, …)`
// functions — one per Rust `WpSupportsLocalization` impl — rather than a Rust-source grep, so
// UniFFI's `Error`→`Exception` renaming is handled for free. `LocalizedErrorParityTest` guards it.
fun appendLocalizedErrorExtensions(bindingFile: File) {
    if (!bindingFile.exists()) return
    val localizer = Regex(
        """fun\s+`?(localize\w+)`?\(\s*`?value`?\s*:\s*(\w+)\s*,\s*`?locale`?\s*:\s*WpLocale\?\s*\)"""
    )
    val extensions = localizer.findAll(bindingFile.readText()).joinToString("\n") { match ->
        val (function, receiver) = match.destructured
        """
        |fun $receiver.localizedDescription(
        |    locale: uniffi.wp_localization.WpLocale =
        |        uniffi.wp_localization.wpLocaleResolve(listOf(java.util.Locale.getDefault().toLanguageTag())),
        |): kotlin.String = $function(this, locale)
        """.trimMargin()
    }
    if (extensions.isNotEmpty()) {
        bindingFile.appendText("\n// Localized error message extensions (generated — parity with Swift).\n$extensions\n")
    }
}

// UniFFI supports generating bindings for multiple crates from a single library file.
// When wp_mobile is built, it includes wp_api as a dependency, so libwp_mobile contains
// metadata for both crates. We generate bindings for each crate from the single library.
val generateUniFFIBindingsTask = tasks.register<Exec>("generateUniFFIBindings") {
    val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot")
    val uniffiGeneratedPath = "${layout.buildDirectory.get()}/generated/source/uniffi/java"
    val nativeLibraryPath = rootProject.ext.get("nativeLibraryPath")!!
    val rustPrimaryModule = rootProject.ext.get("rustPrimaryModule")

    dependsOn(rootProject.tasks.named("cargoBuildLibraryRelease"))
    workingDir(project.rootDir)
    commandLine(
        rootProject.ext.get("cargoBinaryPath")!!,
        "run",
        "--release",
        "--bin",
        "wp_uniffi_bindgen",
        "generate",
        "--no-format",
        "--library",
        nativeLibraryPath,
        "--out-dir",
        uniffiGeneratedPath,
        "--language",
        "kotlin"
    )
    outputs.dir(uniffiGeneratedPath)
    // Re-generate if the interface definition changes.
    inputs.file(nativeLibraryPath)
    // Re-generate if our uniffi-bindgen tooling changes.
    inputs.dir("$cargoProjectRoot/wp_uniffi_bindgen/")
    // Re-generate if our uniffi-bindgen version changes.
    inputs.file("$cargoProjectRoot/Cargo.lock")
    // Re-generate if the module source code changes
    inputs.dir("$cargoProjectRoot/$rustPrimaryModule/")
    // Re-run if the localizedDescription post-processing changes; bump when editing the
    // `appendLocalizedErrorExtensions` logic so a cached binding can't skip the doLast.
    inputs.property("localizedErrorCodegen", "v1")

    // Append the `localizedDescription` extensions after generation, for the same two
    // crates Swift patches (`patch_wp_api` / `patch_wp_mobile`). `wp_localization` and
    // `wp_mobile_cache` ship no localizable types, so they're skipped.
    doLast {
        listOf("wp_api", "wp_mobile").forEach { namespace ->
            appendLocalizedErrorExtensions(
                File("$uniffiGeneratedPath/uniffi/$namespace/$namespace.kt")
            )
        }
    }
}

tasks.named("compileKotlin").configure {
    dependsOn(generateUniFFIBindingsTask)
}
tasks.named("processIntegrationTestResources").configure {
    dependsOn(rootProject.tasks.named("copyDesktopJniLibs"))
    dependsOn(rootProject.tasks.named("copyTestCredentials"))
    dependsOn(rootProject.tasks.named("copyTestMedia"))
    dependsOn(rootProject.tasks.named("copySampleJSON"))
}
tasks.named("sourcesJar").configure {
    dependsOn(generateUniFFIBindingsTask)
}

project.afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("maven") {
                from(components["java"])

                groupId = "rs.wordpress.api"
                artifactId = "kotlin"
                // version is set by "publish-to-s3" plugin
            }
        }
    }
}
