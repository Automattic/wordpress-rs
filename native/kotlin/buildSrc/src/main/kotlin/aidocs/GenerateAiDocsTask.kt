package aidocs

import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import java.io.File

// The only impure step: reads the bindings file, runs the pure parse → generate pipeline, and writes
// the results to disk. All filesystem access lives here; [BindingsParser] and [DocsGenerator] are pure.
abstract class GenerateAiDocsTask : DefaultTask() {
    @get:InputFile
    abstract val generatedBindingsFile: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun generate() {
        val lines = generatedBindingsFile.get().asFile.readLines()
        val parsed = BindingsParser(lines).parse()
        val docs = DocsGenerator(parsed).generate()

        val outputDir = outputDirectory.get().asFile
        // Clean stale output so renamed/removed endpoints don't leave orphan files.
        if (outputDir.exists()) outputDir.deleteRecursively()
        outputDir.mkdirs()

        docs.forEach { doc -> File(outputDir, doc.fileName).writeText(doc.content) }
        logger.lifecycle("AI docs generated: ${outputDir.absolutePath}")
    }
}
