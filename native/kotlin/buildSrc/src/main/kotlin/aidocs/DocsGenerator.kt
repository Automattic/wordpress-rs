package aidocs

/** One generated doc: a logical file name and its markdown content. In-memory only — no disk path. */
data class GeneratedDoc(val fileName: String, val content: String)

// Turns a [ParsedBindings] model into markdown docs. Pure: it returns the docs in memory and never
// touches the filesystem (writing is the task's job).
class DocsGenerator(parsed: ParsedBindings) {
    private val executorInterfaces = parsed.executors
    private val dataClasses = parsed.dataClasses
    private val sealedClasses = parsed.sealedClasses
    private val enumClasses = parsed.enumClasses

    fun generate(): List<GeneratedDoc> {
        val docs = mutableListOf<GeneratedDoc>()

        // WordPress.com stats endpoints are numerous and individually tiny, so we
        // collect them into a single grouped file instead of one file each.
        val (statsExecutors, regularExecutors) = executorInterfaces
            .partition { it.domain.startsWith("stats") }

        val indexEntries = mutableListOf<Pair<String, String>>()

        for (executor in regularExecutors) {
            val fileName = "${executor.domain}.md"
            indexEntries.add(executor.domain to "- [${executor.domain}]($fileName)")
            docs.add(GeneratedDoc(fileName, generateEndpointDoc(executor, level = 1).toString()))
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
            docs.add(GeneratedDoc("stats.md", statsDoc.toString()))
        }

        val index = StringBuilder()
        index.appendLine("# WordPress REST API - Kotlin Bindings Reference")
        index.appendLine()
        index.appendLine("## Endpoints")
        index.appendLine()
        indexEntries.sortedBy { it.first }.forEach { index.appendLine(it.second) }
        docs.add(GeneratedDoc("index.md", index.toString()))

        return docs
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
