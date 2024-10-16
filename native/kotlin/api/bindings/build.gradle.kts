plugins {
    alias(libs.plugins.rustAndroid)
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.publishToS3)
    id("java-library")
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

dependencies {
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

project.afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("maven") {
                from(components["java"])

                groupId = "rs.wordpress.api"
                artifactId = "bindings"
                // version is set by "publish-to-s3" plugin
            }
        }
    }
}
