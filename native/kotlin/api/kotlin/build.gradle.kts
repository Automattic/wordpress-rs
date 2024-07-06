plugins {
    alias(libs.plugins.rustAndroid)
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.publishToS3)
    id("java-library")
    id("jvm-test-suite")
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

@Suppress("UnstableApiUsage")
testing {
    suites {
        val test by getting(JvmTestSuite::class) {
            useJUnit(rootProject.libs.versions.junit.get())
        }

        register<JvmTestSuite>("integrationTest") {
            testType = TestSuiteType.INTEGRATION_TEST

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

                implementation(rootProject.libs.kotlin.test)
                implementation(rootProject.libs.kotlinx.coroutines.test)
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

@Suppress("UnstableApiUsage")
tasks.named("check") {
    dependsOn(testing.suites.named("integrationTest"))
}

dependencies {
    implementation(libs.okhttp)
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

val generateUniFFIBindingsTask = tasks.register<Exec>("generateUniFFIBindings") {
    val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot")
    val uniffiGeneratedPath = "${layout.buildDirectory.get()}/generated/source/uniffi/java"
    val nativeLibraryPath = rootProject.ext.get("nativeLibraryPath")!!
    val rustModuleName = rootProject.ext.get("rustModuleName")

    dependsOn(rootProject.tasks.named("cargoBuildLibraryRelease"))
    workingDir(project.rootDir)
    commandLine(
        "cargo",
        "run",
        "--release",
        "--bin",
        "wp_uniffi_bindgen",
        "generate",
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
    inputs.dir("$cargoProjectRoot/$rustModuleName/")
}


tasks.named("compileKotlin").configure {
    dependsOn(generateUniFFIBindingsTask)
}
tasks.named("processIntegrationTestResources").configure {
    dependsOn(rootProject.tasks.named("copyDesktopJniLibs"))
    dependsOn(rootProject.tasks.named("copyTestCredentials"))
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
