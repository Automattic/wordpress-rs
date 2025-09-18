plugins {
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.androidLibrary) apply false
    alias(libs.plugins.jetbrainsCompose) apply false
    alias(libs.plugins.compose.compiler) apply false
    alias(libs.plugins.kotlinMultiplatform) apply false
    alias(libs.plugins.kotlinJvm) apply false
    alias(libs.plugins.kotlinSerialization) apply false
    alias(libs.plugins.rustAndroid) apply false
    alias(libs.plugins.publishToS3) apply false

    alias(libs.plugins.detekt)
}

allprojects {
    apply(plugin = rootProject.libs.plugins.detekt.get().pluginId)

    detekt {
        toolVersion = rootProject.libs.versions.detekt.plugin.get()
        buildUponDefaultConfig = true
        config.from("${project.rootDir}/config/detekt/detekt.yml")
        allRules = false
    }

    tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach {
        jvmTarget = "1.8"
        reports {
            html.required.set(true)
            xml.required.set(true)
        }

        // Exclude generated bindings
        exclude("**/wp_api.kt")
        exclude("**/wp_localization.kt")
    }

    tasks.withType<io.gitlab.arturbosch.detekt.DetektCreateBaselineTask>().configureEach {
        jvmTarget = "1.8"

        // Exclude generated bindings
        exclude("**/wp_api.kt")
        exclude("**/wp_localization.kt")
    }

    dependencies {
        detektPlugins(rootProject.libs.detekt.formatting)
    }
}

val cargoBinaryPath = resolveBinary("cargo")
val rustcBinaryPath = resolveBinary("rustc")
val cargoProjectRoot = "${project.rootDir}/../.."
val jniLibsPath = "${layout.buildDirectory.get()}/jniLibs/"
val generatedTestResourcesPath = "${layout.buildDirectory.get()}/generatedTestResources/"
val rustModuleName = "wp_api"
val nativeLibraryPath =
    "$cargoProjectRoot/target/release/lib${rustModuleName}${getNativeLibraryExtension()}"

rootProject.ext.set("cargoBinaryPath", cargoBinaryPath)
rootProject.ext.set("rustcBinaryPath", rustcBinaryPath)
rootProject.ext.set("cargoProjectRoot", cargoProjectRoot)
rootProject.ext.set("jniLibsPath", jniLibsPath)
rootProject.ext.set("generatedTestResourcesPath", generatedTestResourcesPath)
rootProject.ext.set("nativeLibraryPath", nativeLibraryPath)
rootProject.ext.set("rustModuleName", rustModuleName)

setupJniAndBindings()

// Separated as a function to have everything in a scope and keep it contained
fun setupJniAndBindings() {
    val nativeLibraryPath =
        "$cargoProjectRoot/target/release/lib${rustModuleName}${getNativeLibraryExtension()}"

    val cargoBuildLibraryReleaseTask = tasks.register<Exec>("cargoBuildLibraryRelease") {
        workingDir(rootProject.ext.get("cargoProjectRoot")!!)
        commandLine(cargoBinaryPath, "build", "--package", rustModuleName, "--release")
        // No inputs.dir added, because we want to always re-run this task and let Cargo handle caching
    }

    tasks.register<Copy>("copyDesktopJniLibs") {
        dependsOn(cargoBuildLibraryReleaseTask)
        from(nativeLibraryPath)
        into(jniLibsPath)
    }

    tasks.register<Delete>("deleteGeneratedTestResources") {
        delete = setOf(generatedTestResourcesPath)
    }

    tasks.register<Copy>("copyTestCredentials") {
        dependsOn(tasks.named("deleteGeneratedTestResources"))
        from("$cargoProjectRoot/test_credentials.json")
        into(generatedTestResourcesPath)
    }

    tasks.register<Copy>("copyTestMedia") {
        dependsOn(tasks.named("deleteGeneratedTestResources"))
        from("$cargoProjectRoot/test-data/test_media.jpg")
        into(generatedTestResourcesPath)
    }

    tasks.register<Copy>("copySampleJSON") {
        dependsOn(tasks.named("deleteGeneratedTestResources"))
        from("$cargoProjectRoot/test-data/integration-test-responses/localhost-json-root.json")
        into(generatedTestResourcesPath)
    }
}

fun resolveBinary(name: String): String {
    val process = ProcessBuilder().apply {
        command(listOf("which", name))
    }.start()

    process.waitFor()

    return process.inputReader().readText().trim()
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

