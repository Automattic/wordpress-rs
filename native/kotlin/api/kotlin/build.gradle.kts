plugins {
    alias(libs.plugins.rustAndroid)
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.kotlinSerialization)
    alias(libs.plugins.publishToS3)
    id("java-library")
    id("jvm-test-suite")
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

// Generated AI reference docs are a build output (not committed). Single source of truth for the path.
val aiDocsDir = layout.buildDirectory.dir("ai-docs/ai-reference")

val generateAiDocs = tasks.register<GenerateAiDocsTask>("generateAiDocs") {
    dependsOn(generateUniFFIBindingsTask)
    generatedBindingsFile.set(
        layout.buildDirectory.file("generated/source/uniffi/java/uniffi/wp_api/wp_api.kt")
    )
    outputDirectory.set(aiDocsDir)
}

// Derive the publisher's source from the generator's output so zipAiDocs implicitly depends on generateAiDocs.
aiDocs {
    from(generateAiDocs.flatMap { it.outputDirectory })
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
