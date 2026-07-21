import aidocs.GenerateAiDocsTask
import org.gradle.api.publish.PublishingExtension
import org.gradle.api.publish.maven.MavenPublication
import org.gradle.api.tasks.bundling.Zip

// Self-contained "AI docs" build logic. Everything for the feature lives here so it can be added or
// removed by a single `id("ai-docs")` line in a module's build script:
//   1. generate the per-endpoint markdown reference from the UniFFI bindings,
//   2. zip it reproducibly,
//   3. attach the zip to the module's Maven publication under the `ai-docs` classifier.

// Generated AI reference docs are a build output (not committed). Single source of truth for the path.
val aiDocsDir = layout.buildDirectory.dir("ai-docs/ai-reference")

val generateAiDocs = tasks.register<GenerateAiDocsTask>("generateAiDocs") {
    // Depend by name: `generateUniFFIBindings` is registered later in the module's build script,
    // after this plugin is applied, so we can't hold a typed reference to it here.
    dependsOn("generateUniFFIBindings")
    generatedBindingsFile.set(
        layout.buildDirectory.file("generated/source/uniffi/java/uniffi/wp_api/wp_api.kt")
    )
    outputDirectory.set(aiDocsDir)
}

// Gradle's built-in Zip gives a reproducible, cross-platform archive for free (forward-slash
// entries, stable order, fixed timestamps). `from(provider)` makes this task depend on
// `generateAiDocs` and wires `builtBy` into the published artifact automatically.
val zipAiDocs = tasks.register<Zip>("zipAiDocs") {
    from(generateAiDocs.flatMap { it.outputDirectory })
    // Own subdir so this archive doesn't collide with a consumer's `build/ai-docs` unpack dir.
    destinationDirectory.set(layout.buildDirectory.dir("ai-docs-archive"))
    archiveFileName.set("ai-docs.zip")
    isReproducibleFileOrder = true
    isPreserveFileTimestamps = false
}

// Attach the zip to the module's Maven publication(s). `configureEach` is lazy, so it also applies
// to the `maven` publication created later in the build script's `afterEvaluate` block.
pluginManager.withPlugin("maven-publish") {
    the<PublishingExtension>().publications.withType<MavenPublication>().configureEach {
        artifact(zipAiDocs.flatMap { it.archiveFile }) {
            classifier = "ai-docs"
            extension = "zip"
        }
    }
}
