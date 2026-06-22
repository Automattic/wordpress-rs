import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import java.io.File

abstract class GenerateAiDocsTask : DefaultTask() {
    @get:InputFile
    abstract val generatedBindingsFile: RegularFileProperty

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @TaskAction
    fun generate() {
        val sourceFile = generatedBindingsFile.get().asFile
        val outputDir = outputDirectory.get().asFile
        // Clean stale output so renamed/removed endpoints don't leave orphan files.
        if (outputDir.exists()) outputDir.deleteRecursively()
        outputDir.mkdirs()

        val lines = sourceFile.readLines()
        val parser = BindingsParser(lines)

        val executorInterfaces = parser.parseExecutorInterfaces()
        val dataClasses = parser.parseDataClasses()
        val sealedClasses = parser.parseSealedClasses()
        val enumClasses = parser.parseEnumClasses()

        val generator = DocsGenerator(
            executorInterfaces = executorInterfaces,
            dataClasses = dataClasses,
            sealedClasses = sealedClasses,
            enumClasses = enumClasses
        )

        generator.generate(outputDir)
        logger.lifecycle("AI docs generated: ${outputDir.absolutePath}")
    }
}

data class MethodSignature(
    val name: String,
    val params: List<Pair<String, String>>,
    val returnType: String,
    val isSuspend: Boolean
)

data class ExecutorInterface(
    val name: String,
    val domain: String,
    val methods: List<MethodSignature>
)

data class DataClassInfo(
    val name: String,
    val fields: List<Triple<String, String, String?>>
)

data class SealedClassInfo(
    val name: String,
    val variants: List<String>
)

data class EnumClassInfo(
    val name: String,
    val variants: List<String>
)

class BindingsParser(private val lines: List<String>) {

    private val identifierRegex = Regex("^[A-Za-z][A-Za-z0-9_]*$")

    fun parseExecutorInterfaces(): List<ExecutorInterface> {
        val interfaces = mutableListOf<ExecutorInterface>()
        var i = 0

        while (i < lines.size) {
            val line = lines[i]
            if (line.startsWith("public interface ") && line.endsWith("RequestExecutorInterface {")) {
                val name = line.removePrefix("public interface ").removeSuffix(" {")
                val domain = name.removeSuffix("RequestExecutorInterface")
                    .replaceFirstChar { it.lowercase() }

                val methods = mutableListOf<MethodSignature>()
                i++

                while (i < lines.size && lines[i] != "}") {
                    val methodLine = lines[i].trim()
                    val parsed = parseMethodSignature(methodLine)
                    if (parsed != null) methods.add(parsed)
                    i++
                }

                interfaces.add(ExecutorInterface(name, domain, methods))
            }
            i++
        }

        return interfaces
    }

    fun parseDataClasses(): Map<String, DataClassInfo> {
        val classes = mutableMapOf<String, DataClassInfo>()
        var i = 0

        while (i < lines.size) {
            val line = lines[i]
            if (line.startsWith("data class ") && line.contains("(")) {
                val name = line.removePrefix("data class ").substringBefore(" (").trim()
                val fields = mutableListOf<Triple<String, String, String?>>()
                i++

                while (i < lines.size) {
                    val fieldLine = lines[i].trim()
                    if (fieldLine.startsWith("val ")) {
                        val parsed = parseField(fieldLine)
                        if (parsed != null) fields.add(parsed)
                    } else if (fieldLine.startsWith(")") || fieldLine == "): Disposable{") {
                        break
                    }
                    i++
                }

                if (fields.isNotEmpty()) {
                    classes[name] = DataClassInfo(name, fields)
                }
            }
            i++
        }

        return classes
    }

    fun parseSealedClasses(): Map<String, SealedClassInfo> {
        val classes = mutableMapOf<String, SealedClassInfo>()
        var i = 0

        while (i < lines.size) {
            val line = lines[i]
            if (line.startsWith("sealed class ") && line.endsWith("{")) {
                val name = line.removePrefix("sealed class ").substringBefore(" ").trim()
                val variants = mutableListOf<String>()
                i++

                while (i < lines.size && lines[i] != "}") {
                    val variantLine = lines[i].trim()
                    if (variantLine.startsWith("data class ") || variantLine.startsWith("object ")) {
                        val variantName = if (variantLine.startsWith("data class ")) {
                            variantLine.removePrefix("data class ").substringBefore("(").trim()
                        } else {
                            variantLine.removePrefix("object ").substringBefore(" ").substringBefore(":").trim()
                        }
                        variants.add(variantName)
                    }
                    i++
                }

                if (variants.isNotEmpty()) {
                    classes[name] = SealedClassInfo(name, variants)
                }
            }
            i++
        }

        return classes
    }

    fun parseEnumClasses(): Map<String, EnumClassInfo> {
        val classes = mutableMapOf<String, EnumClassInfo>()
        var i = 0

        while (i < lines.size) {
            val line = lines[i]
            if (line.startsWith("enum class ") && line.contains("{")) {
                val name = line.removePrefix("enum class ").substringBefore("(").substringBefore(" {").trim()
                val variants = mutableListOf<String>()
                i++

                while (i < lines.size && lines[i].trim() != "}") {
                    val variantLine = lines[i].trim()
                    // Enum entries are terminated by ';' when the enum also has methods.
                    // Capture the final entry on that line, then stop before the methods.
                    val entryListEnd = variantLine.endsWith(";")
                    if (variantLine.isNotEmpty() && !variantLine.startsWith("//")) {
                        val variantName = variantLine.removeSuffix(",").removeSuffix(";").trim()
                            .substringBefore("(").trim()
                        if (variantName.matches(identifierRegex)) {
                            variants.add(variantName)
                        }
                    }
                    if (entryListEnd) break
                    i++
                }

                if (variants.isNotEmpty()) {
                    classes[name] = EnumClassInfo(name, variants)
                }
            }
            i++
        }

        return classes
    }

    private fun parseMethodSignature(line: String): MethodSignature? {
        if (!line.startsWith("fun ") && !line.startsWith("suspend fun ")) return null
        if (line.contains("companion object")) return null

        val isSuspend = line.startsWith("suspend ")
        val withoutPrefix = if (isSuspend) line.removePrefix("suspend fun ") else line.removePrefix("fun ")

        val name = withoutPrefix.substringBefore("(").removeSurrounding("`")
        val paramsStr = withoutPrefix.substringAfter("(").substringBefore(")")
        val returnType = withoutPrefix.substringAfter("): ", "Unit")

        val params = if (paramsStr.isBlank()) {
            emptyList()
        } else {
            paramsStr.split(", ").mapNotNull { param ->
                val parts = param.split(": ", limit = 2)
                if (parts.size == 2) {
                    val pName = parts[0].removeSurrounding("`").trim()
                    val pType = parts[1].trim()
                    pName to pType
                } else null
            }
        }

        return MethodSignature(name, params, returnType, isSuspend)
    }

    private fun parseField(line: String): Triple<String, String, String?>? {
        val withoutVal = line.removePrefix("val ").trim()
        val nameRaw = withoutVal.substringBefore(":").removeSurrounding("`").trim()
        val rest = withoutVal.substringAfter(": ", "")
        if (rest.isEmpty()) return null

        val type = rest.substringBefore(" =").substringBefore(" \n").trim()
            .removeSuffix(",").trim()
        val default = if (rest.contains(" = ")) {
            rest.substringAfter(" = ").substringBefore(" \n").removeSuffix(",").trim()
        } else null

        return Triple(nameRaw, cleanType(type), default)
    }

    private fun cleanType(type: String): String = type
        .replace("kotlin.String", "String")
        .replace("kotlin.Boolean", "Boolean")
        .replace("kotlin.UInt", "UInt")
        .replace("kotlin.Int", "Int")
        .replace("kotlin.Long", "Long")
        .replace("kotlin.ULong", "ULong")
        .replace("kotlin.UByte", "UByte")
        .replace("kotlin.Double", "Double")
}

class DocsGenerator(
    private val executorInterfaces: List<ExecutorInterface>,
    private val dataClasses: Map<String, DataClassInfo>,
    private val sealedClasses: Map<String, SealedClassInfo>,
    private val enumClasses: Map<String, EnumClassInfo>
) {
    fun generate(outputDir: File) {
        val index = StringBuilder()
        index.appendLine("# WordPress REST API - Kotlin Bindings Reference")
        index.appendLine()
        index.appendLine("## Endpoints")
        index.appendLine()

        // WordPress.com stats endpoints are numerous and individually tiny, so we
        // collect them into a single grouped file instead of one file each.
        val (statsExecutors, regularExecutors) = executorInterfaces
            .partition { it.domain.startsWith("stats") }

        val indexEntries = mutableListOf<Pair<String, String>>()

        for (executor in regularExecutors) {
            val filename = "${executor.domain}.md"
            indexEntries.add(executor.domain to "- [${executor.domain}]($filename)")
            File(outputDir, filename).writeText(generateEndpointDoc(executor, level = 1).toString())
        }

        if (statsExecutors.isNotEmpty()) {
            indexEntries.add("stats" to "- [stats](stats.md)")
            val statsDoc = StringBuilder()
            statsDoc.appendLine("# stats")
            statsDoc.appendLine()
            statsDoc.appendLine("WordPress.com stats endpoints.")
            for (executor in statsExecutors.sortedBy { it.domain }) {
                statsDoc.appendLine()
                statsDoc.append(generateEndpointDoc(executor, level = 2))
            }
            File(outputDir, "stats.md").writeText(statsDoc.toString())
        }

        indexEntries.sortedBy { it.first }.forEach { index.appendLine(it.second) }

        File(outputDir, "index.md").writeText(index.toString())
    }

    private fun generateEndpointDoc(executor: ExecutorInterface, level: Int): StringBuilder {
        val h1 = "#".repeat(level)
        val h2 = "#".repeat(level + 1)
        val h3 = "#".repeat(level + 2)

        val doc = StringBuilder()
        doc.appendLine("$h1 ${executor.domain}")
        doc.appendLine()

        val apiMethods = executor.methods.filter {
            it.name != "cancel" && it.name != "fetchAuthenticationState"
        }

        doc.appendLine("$h2 Methods")
        doc.appendLine()
        for (method in apiMethods) {
            val params = method.params.joinToString(", ") { "${it.first}: ${it.second}" }
            doc.appendLine("- `${method.name}($params): ${method.returnType}`")
        }

        val referencedTypes = collectReferencedTypes(apiMethods)
        val relevantDataClasses = referencedTypes
            .mapNotNull { dataClasses[it] }
            .filter { !it.name.endsWith("Response") || it.fields.any { f -> f.first != "data" && f.first != "headerMap" } }

        val paramsClasses = relevantDataClasses.filter { it.name.endsWith("Params") }
        val entityClasses = relevantDataClasses.filter { !it.name.endsWith("Params") && !it.name.endsWith("Response") }

        if (paramsClasses.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("$h2 Parameters")
            for (cls in paramsClasses) {
                doc.appendLine()
                writeDataClass(doc, cls, h3)
            }
        }

        if (entityClasses.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("$h2 Types")
            for (cls in entityClasses) {
                doc.appendLine()
                writeDataClass(doc, cls, h3)
            }
        }

        val relevantEnums = referencedTypes.mapNotNull { enumClasses[it] }
        val relevantSealed = referencedTypes.mapNotNull { sealedClasses[it] }

        if (relevantEnums.isNotEmpty() || relevantSealed.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("$h2 Enums")
            for (enum in relevantEnums) {
                doc.appendLine()
                doc.appendLine("$h3 ${enum.name}")
                doc.appendLine("Variants: ${enum.variants.joinToString(", ") { "`$it`" }}")
            }
            for (sealed in relevantSealed) {
                doc.appendLine()
                doc.appendLine("$h3 ${sealed.name}")
                doc.appendLine("Variants: ${sealed.variants.joinToString(", ") { "`$it`" }}")
            }
        }

        return doc
    }

    private fun writeDataClass(doc: StringBuilder, cls: DataClassInfo, heading: String) {
        doc.appendLine("$heading ${cls.name}")
        doc.appendLine("| Field | Type | Default |")
        doc.appendLine("|-------|------|---------|")
        for ((name, type, default) in cls.fields) {
            doc.appendLine("| `$name` | `$type` | ${default ?: ""} |")
        }
    }

    private fun collectReferencedTypes(methods: List<MethodSignature>): Set<String> {
        val types = mutableSetOf<String>()
        for (method in methods) {
            for ((_, type) in method.params) {
                types.add(extractTypeName(type))
            }
            types.add(extractTypeName(method.returnType))
        }

        val expanded = mutableSetOf<String>()
        expanded.addAll(types)
        for (type in types) {
            val dc = dataClasses[type]
            if (dc != null) {
                for ((_, fieldType, _) in dc.fields) {
                    expanded.add(extractTypeName(fieldType))
                }
            }
        }

        return expanded
    }

    private fun extractTypeName(type: String): String = type
        .removePrefix("List<").removeSuffix(">")
        .removeSuffix("?")
        .trim()
}
