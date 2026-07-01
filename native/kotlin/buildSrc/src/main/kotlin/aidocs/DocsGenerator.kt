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
    private val freeFunctions = parsed.freeFunctions
    private val typeNameRegex = Regex("[A-Za-z_][A-Za-z0-9_]*")

    fun generate(): List<GeneratedDoc> {
        val endpoints = executorInterfaces.map { executor ->
            GeneratedDoc("${executor.domain}.md", generateEndpointDoc(executor))
        }
        val functions = if (freeFunctions.isEmpty()) emptyList() else listOf(GeneratedDoc("functions.md", functionsDoc()))
        return endpoints + functions + GeneratedDoc("index.md", buildIndex())
    }

    private fun buildIndex(): String = buildString {
        appendLine("# WordPress REST API - Kotlin Bindings Reference")
        appendLine()
        appendLine("## Endpoints")
        appendLine()
        executorInterfaces.map { it.domain }.sorted().forEach { domain ->
            appendLine("- [$domain]($domain.md)")
        }
        if (freeFunctions.isNotEmpty()) {
            appendLine()
            appendLine("## Functions")
            appendLine()
            appendLine("- [functions](functions.md)")
        }
    }

    private fun functionsDoc(): String = buildString {
        appendLine("# functions")
        appendLine()
        freeFunctions.sortedBy { it.name }.forEach { appendLine(signatureLine(it)) }
    }

    // Flat structure: the interface's method list, then one block per referenced type, all separated by
    // `---` so a consumer can split the file into blocks. The methods live under a `## interface <Name>`
    // header so they're owned by their declaring type, matching the `## <kind> <Name>` type blocks below.
    // Every type named in a signature or field has its own block, so the doc is self-contained.
    private fun generateEndpointDoc(executor: ExecutorInterface): String {
        val methods = executor.methods.filterNot { it.name in EXCLUDED_METHODS }
        val interfaceBlock = buildString {
            appendLine("# ${executor.domain}")
            appendLine()
            appendLine("## interface ${executor.name}")
            appendLine()
            methods.forEach { appendLine(signatureLine(it)) }
        }.trimEnd()
        val typeBlocks = collectReferencedTypes(methods).mapNotNull { typeBlock(it)?.trimEnd() }
        return (listOf(interfaceBlock) + typeBlocks).joinToString("\n\n---\n\n") + "\n"
    }

    private fun signatureLine(function: MethodSignature): String {
        val params = function.params.joinToString(", ") { "${it.name}: ${it.type}" }
        return "- `${function.name}($params): ${function.returnType}`"
    }

    private fun typeBlock(typeName: String): String? =
        dataClasses[typeName]?.let { dataClassBlock(it) }
            ?: enumClasses[typeName]?.let { enumBlock(it) }
            ?: sealedClasses[typeName]?.let { sealedBlock(it) }

    private fun dataClassBlock(cls: DataClassInfo): String = buildString {
        appendLine("## data class ${cls.name}")
        appendLine()
        appendLine("| Field | Type | Default |")
        appendLine("|-------|------|---------|")
        cls.fields.forEach { (name, type, default) ->
            appendLine("| `$name` | `$type` | ${default ?: ""} |")
        }
    }

    private fun enumBlock(enum: EnumClassInfo): String = buildString {
        appendLine("## enum class ${enum.name}")
        appendLine()
        appendLine("Variants: ${enum.variants.joinToString(", ") { "`$it`" }}")
    }

    private fun sealedBlock(sealed: SealedClassInfo): String = buildString {
        appendLine("## sealed class ${sealed.name}")
        appendLine()
        sealed.variants.forEach { appendLine(variantLine(it)) }
    }

    private fun variantLine(variant: SealedVariant): String =
        if (variant.fields.isEmpty()) {
            "- `${variant.name}`"
        } else {
            "- `${variant.name}(${variant.fields.joinToString(", ") { "${it.name}: ${it.type}" }})`"
        }

    private fun collectReferencedTypes(methods: List<MethodSignature>): Set<String> {
        val seeds = methods.flatMap { method ->
            method.params.flatMap { extractTypeNames(it.type) } + extractTypeNames(method.returnType)
        }.toSet()
        return reachableTypes(seeds)
    }

    private tailrec fun reachableTypes(frontier: Set<String>, seen: Set<String> = emptySet()): Set<String> {
        if (frontier.isEmpty()) return seen
        val nextSeen = seen + frontier
        val discovered = frontier.flatMap { fieldTypesOf(it) }.toSet()
        return reachableTypes(discovered - nextSeen, nextSeen)
    }

    // The types a given type points at: a data class's field types, or a sealed type's variant field
    // types. Following both keeps the closure complete through sealed hierarchies too.
    private fun fieldTypesOf(type: String): List<String> =
        dataClasses[type]?.fields.orEmpty().flatMap { extractTypeNames(it.type) } +
            sealedClasses[type]?.variants.orEmpty().flatMap { variant ->
                variant.fields.flatMap { extractTypeNames(it.type) }
            }

    // Every identifier in a type expression, e.g. `Map<String, List<JsonValue>>?` -> [Map, String,
    // List, JsonValue]. Built-ins and generics simply don't resolve to a documented type.
    private fun extractTypeNames(type: String): List<String> = typeNameRegex.findAll(type).map { it.value }.toList()

    private companion object {
        val EXCLUDED_METHODS = setOf("cancel", "fetchAuthenticationState")
    }
}
