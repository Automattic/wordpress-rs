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

    private fun buildIndex(): String = buildString {
        appendLine("# WordPress REST API - Kotlin Bindings Reference")
        appendLine()
        appendLine("## Endpoints")
        appendLine()
        executorInterfaces.map { it.domain }.sorted().forEach { domain ->
            appendLine("- [$domain]($domain.md)")
        }
    }

    private fun generateEndpointDoc(executor: ExecutorInterface): String {
        val methods = executor.methods.filterNot { it.name in EXCLUDED_METHODS }
        val referencedTypes = collectReferencedTypes(methods)

        val documentedClasses = referencedTypes.mapNotNull { dataClasses[it] }.filter { isDocumentedType(it) }
        val paramsClasses = documentedClasses.filter { it.name.endsWith(PARAMS_SUFFIX) }
        val entityClasses = documentedClasses.filter { !it.name.endsWith(PARAMS_SUFFIX) && !it.name.endsWith(RESPONSE_SUFFIX) }
        val enums = referencedTypes.mapNotNull { enumClasses[it] }
        val sealedTypes = referencedTypes.mapNotNull { sealedClasses[it] }

        return buildString {
            appendLine("# ${executor.domain}")
            appendLine()
            append(methodsSection(methods))
            append(classesSection("Parameters", paramsClasses))
            append(classesSection("Types", entityClasses))
            append(enumsSection(enums, sealedTypes))
        }
    }

    private fun methodsSection(methods: List<MethodSignature>): String = buildString {
        appendLine("## Methods")
        appendLine()
        methods.forEach { method ->
            val params = method.params.joinToString(", ") { "${it.name}: ${it.type}" }
            appendLine("- `${method.name}($params): ${method.returnType}`")
        }
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

    // A `*Response` wrapper whose only fields are the data/headerMap envelope adds nothing to the docs.
    private fun isDocumentedType(cls: DataClassInfo): Boolean =
        !cls.name.endsWith(RESPONSE_SUFFIX) || cls.fields.any { it.name !in RESPONSE_ENVELOPE_FIELDS }

    private companion object {
        val EXCLUDED_METHODS = setOf("cancel", "fetchAuthenticationState")
        val RESPONSE_ENVELOPE_FIELDS = setOf("data", "headerMap")
        const val RESPONSE_SUFFIX = "Response"
        const val PARAMS_SUFFIX = "Params"
    }
}
