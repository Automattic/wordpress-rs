plugins {
    alias(libs.plugins.rustAndroid)
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.publishToS3)
    id("java-library")
    id("jvm-test-suite")
}

val jniLibsPath = "${layout.buildDirectory.get()}/jniLibs/"
val generatedTestResourcesPath = "${layout.buildDirectory.get()}/generatedTestResources/"
val cargoProjectRoot = rootProject.ext.get("cargoProjectRoot")!!

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
                    setSrcDirs(listOf(jniLibsPath, generatedTestResourcesPath))
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

repositories {
    mavenCentral()
    google()
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

setupJniAndBindings()

// Separated as a function to have everything in a scope and keep it contained
fun setupJniAndBindings() {
    val moduleName = "wp_api"
    val nativeLibraryPath = "$cargoProjectRoot/target/release/lib${moduleName}${getNativeLibraryExtension()}"
    val uniffiGeneratedPath = "${layout.buildDirectory.get()}/generated/source/uniffi/java"

    val cargoBuildLibraryReleaseTask = tasks.register<Exec>("cargoBuildLibraryRelease") {
        workingDir(cargoProjectRoot)
        commandLine("cargo", "build", "--package", moduleName, "--release")
        // No inputs.dir added, because we want to always re-run this task and let Cargo handle caching
    }

    val generateUniFFIBindingsTask = tasks.register<Exec>("generate_${moduleName}_UniFFIBindings") {
        dependsOn(cargoBuildLibraryReleaseTask)
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
        inputs.dir("$cargoProjectRoot/${moduleName}/")
    }

    tasks.named("compileKotlin").configure {
        dependsOn(generateUniFFIBindingsTask)
    }
    val copyDesktopJniLibsTask = tasks.register<Copy>("copyDesktopJniLibs") {
        dependsOn(cargoBuildLibraryReleaseTask)
        from(nativeLibraryPath)
        into(jniLibsPath)
    }
    val copyTestCredentialsTask = tasks.register<Copy>("copyTestCredentials") {
        from("$cargoProjectRoot/test_credentials")
        into(generatedTestResourcesPath)
    }
    tasks.named("processIntegrationTestResources").configure {
        dependsOn(copyDesktopJniLibsTask)
        dependsOn(copyTestCredentialsTask)
    }
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

fun getNativeLibraryExtension(): String {
    val currentOS = org.gradle.internal.os.OperatingSystem.current()
    return if (currentOS.isLinux) {
        ".so"
    } else if (currentOS.isMacOsX) {
        ".dylib"
    } else {
        throw GradleException("Unsupported Operating System: $currentOS")
    }
}
