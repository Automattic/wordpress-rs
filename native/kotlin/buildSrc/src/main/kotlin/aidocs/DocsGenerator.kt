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
        appendLine("Top-level functions exported by the library.")
        appendLine()
        freeFunctions.sortedBy { it.name }.forEach { appendLine(signatureLine(it)) }
    }

    private fun generateEndpointDoc(executor: ExecutorInterface): String {
        val methods = executor.methods.filterNot { it.name in EXCLUDED_METHODS }
        val referencedTypes = collectReferencedTypes(methods)

        val referencedClasses = referencedTypes.mapNotNull { dataClasses[it] }
        val paramsClasses = referencedClasses.filter { it.name.endsWith(PARAMS_SUFFIX) }
        val typeClasses = referencedClasses.filter { !it.name.endsWith(PARAMS_SUFFIX) }
        val enums = referencedTypes.mapNotNull { enumClasses[it] }
        val sealedTypes = referencedTypes.mapNotNull { sealedClasses[it] }

        return buildString {
            appendLine("# ${executor.domain}")
            appendLine()
            append(methodsSection(methods))
            append(classesSection("Parameters", paramsClasses))
            append(classesSection("Types", typeClasses))
            append(enumsSection(enums, sealedTypes))
        }
    }

    private fun methodsSection(methods: List<MethodSignature>): String = buildString {
        appendLine("## Methods")
        appendLine()
        methods.forEach { appendLine(signatureLine(it)) }
    }

    private fun signatureLine(function: MethodSignature): String {
        val params = function.params.joinToString(", ") { "${it.name}: ${it.type}" }
        return "- `${function.name}($params): ${function.returnType}`"
    }

    private fun classesSection(title: String, classes: List<DataClassInfo>): String {
        if (classes.isEmpty()) return ""
        return buildString {
            appendLine()
            appendLine("## $title")
            classes.forEach { cls ->
                appendLine()
                append(dataClassTable(cls, "###"))
            }
        }
    }

    private fun enumsSection(enums: List<EnumClassInfo>, sealedTypes: List<SealedClassInfo>): String {
        if (enums.isEmpty() && sealedTypes.isEmpty()) return ""
        return buildString {
            appendLine()
            appendLine("## Enums")
            enums.forEach { enum ->
                appendLine()
                appendLine("### ${enum.name}")
                appendLine("Variants: ${enum.variants.joinToString(", ") { "`$it`" }}")
            }
            sealedTypes.forEach { sealedType ->
                appendLine()
                appendLine("### ${sealedType.name}")
                appendLine("Variants: ${sealedType.variants.joinToString(", ") { "`$it`" }}")
            }
        }
    }

    private fun dataClassTable(cls: DataClassInfo, heading: String): String = buildString {
        appendLine("$heading ${cls.name}")
        appendLine("| Field | Type | Default |")
        appendLine("|-------|------|---------|")
        cls.fields.forEach { (name, type, default) ->
            appendLine("| `$name` | `$type` | ${default ?: ""} |")
        }
    }

    // All data-class types reachable from these methods: their parameter/return types, then those
    // types' field types, transitively. Keeping the closure complete makes each endpoint doc
    // self-contained — every type it names has its own table, so a consuming session never has to fall
    // back to the bindings source to resolve one.
    private fun collectReferencedTypes(methods: List<MethodSignature>): Set<String> {
        val seeds = methods.flatMap { method ->
            method.params.map { extractTypeName(it.type) } + extractTypeName(method.returnType)
        }.toSet()
        return reachableTypes(seeds)
    }

    private tailrec fun reachableTypes(frontier: Set<String>, seen: Set<String> = emptySet()): Set<String> {
        if (frontier.isEmpty()) return seen
        val nextSeen = seen + frontier
        val discovered = frontier
            .flatMap { type -> dataClasses[type]?.fields.orEmpty().map { extractTypeName(it.type) } }
            .toSet()
        return reachableTypes(discovered - nextSeen, nextSeen)
    }

    // Strip nullability first so `List<Foo>?` yields `Foo`, not `Foo>`; the inner element can itself be
    // nullable (`List<Foo?>`), hence the trailing `removeSuffix("?")`.
    private fun extractTypeName(type: String): String = type
        .removeSuffix("?")
        .removePrefix("List<").removeSuffix(">")
        .removeSuffix("?")
        .trim()

    private companion object {
        val EXCLUDED_METHODS = setOf("cancel", "fetchAuthenticationState")
        const val PARAMS_SUFFIX = "Params"
    }
}
