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
        val endpoints = executorInterfaces.map { executor ->
            GeneratedDoc("${executor.domain}.md", generateEndpointDoc(executor))
        }
        return endpoints + GeneratedDoc("index.md", buildIndex())
    }

    private fun buildIndex(): String {
        val index = StringBuilder()
        index.appendLine("# WordPress REST API - Kotlin Bindings Reference")
        index.appendLine()
        index.appendLine("## Endpoints")
        index.appendLine()
        executorInterfaces.map { it.domain }.sorted().forEach { domain ->
            index.appendLine("- [$domain]($domain.md)")
        }
        return index.toString()
    }

    private fun generateEndpointDoc(executor: ExecutorInterface): String {
        val doc = StringBuilder()
        doc.appendLine("# ${executor.domain}")
        doc.appendLine()

        val apiMethods = executor.methods.filter {
            it.name != "cancel" && it.name != "fetchAuthenticationState"
        }

        doc.appendLine("## Methods")
        doc.appendLine()
        for (method in apiMethods) {
            val params = method.params.joinToString(", ") { "${it.name}: ${it.type}" }
            doc.appendLine("- `${method.name}($params): ${method.returnType}`")
        }

        val referencedTypes = collectReferencedTypes(apiMethods)
        val relevantDataClasses = referencedTypes
            .mapNotNull { dataClasses[it] }
            .filter { !it.name.endsWith("Response") || it.fields.any { f -> f.name != "data" && f.name != "headerMap" } }

        val paramsClasses = relevantDataClasses.filter { it.name.endsWith("Params") }
        val entityClasses = relevantDataClasses.filter { !it.name.endsWith("Params") && !it.name.endsWith("Response") }

        if (paramsClasses.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("## Parameters")
            for (cls in paramsClasses) {
                doc.appendLine()
                doc.append(dataClassTable(cls, "###"))
            }
        }

        if (entityClasses.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("## Types")
            for (cls in entityClasses) {
                doc.appendLine()
                doc.append(dataClassTable(cls, "###"))
            }
        }

        val relevantEnums = referencedTypes.mapNotNull { enumClasses[it] }
        val relevantSealed = referencedTypes.mapNotNull { sealedClasses[it] }

        if (relevantEnums.isNotEmpty() || relevantSealed.isNotEmpty()) {
            doc.appendLine()
            doc.appendLine("## Enums")
            for (enum in relevantEnums) {
                doc.appendLine()
                doc.appendLine("### ${enum.name}")
                doc.appendLine("Variants: ${enum.variants.joinToString(", ") { "`$it`" }}")
            }
            for (sealed in relevantSealed) {
                doc.appendLine()
                doc.appendLine("### ${sealed.name}")
                doc.appendLine("Variants: ${sealed.variants.joinToString(", ") { "`$it`" }}")
            }
        }

        return doc.toString()
    }

    private fun dataClassTable(cls: DataClassInfo, heading: String): String = buildString {
        appendLine("$heading ${cls.name}")
        appendLine("| Field | Type | Default |")
        appendLine("|-------|------|---------|")
        cls.fields.forEach { (name, type, default) ->
            appendLine("| `$name` | `$type` | ${default ?: ""} |")
        }
    }

    private fun collectReferencedTypes(methods: List<MethodSignature>): Set<String> {
        val direct = methods.flatMap { method ->
            method.params.map { extractTypeName(it.type) } + extractTypeName(method.returnType)
        }.toSet()
        val nested = direct.flatMap { type ->
            dataClasses[type]?.fields.orEmpty().map { extractTypeName(it.type) }
        }
        return direct + nested
    }

    private fun extractTypeName(type: String): String = type
        .removePrefix("List<").removeSuffix(">")
        .removeSuffix("?")
        .trim()
}
