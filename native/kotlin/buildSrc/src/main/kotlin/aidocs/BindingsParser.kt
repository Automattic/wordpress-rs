package aidocs

// A matched top-level declaration: its header line plus every line that follows it. Callers slice the
// body down to the lines they care about.
private data class Decl(val header: String, val body: List<String>)

// Parses the UniFFI-generated `wp_api.kt` text into a [ParsedBindings] model. Pure: it operates only
// on the lines it is given and performs no I/O.
class BindingsParser(private val lines: List<String>) {

    private val identifierRegex = Regex("^[A-Za-z][A-Za-z0-9_]*$")
    private val kotlinPackagePrefix = Regex("\\bkotlin\\.")
    private val freeFunctionRegex = Regex("(suspend )?fun `\\w+`\\(")

    fun parse(): ParsedBindings = ParsedBindings(
        executors = parseExecutorInterfaces(),
        dataClasses = parseDataClasses(),
        sealedClasses = parseSealedClasses(),
        enumClasses = parseEnumClasses(),
        freeFunctions = parseFreeFunctions()
    )

    fun parseExecutorInterfaces(): List<ExecutorInterface> =
        blocks { it.startsWith(INTERFACE_PREFIX) && it.endsWith("$EXECUTOR_SUFFIX {") }
            .map { (header, body) ->
                val name = header.removePrefix(INTERFACE_PREFIX).removeSuffix(" {")
                val domain = name.removeSuffix(EXECUTOR_SUFFIX).replaceFirstChar { c -> c.lowercase() }
                val methods = body.takeWhile { it != "}" }.mapNotNull { parseMethodSignature(it.trim()) }
                ExecutorInterface(name, domain, methods)
            }

    fun parseDataClasses(): Map<String, DataClassInfo> =
        blocks { it.startsWith(DATA_CLASS_PREFIX) && it.contains("(") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix(DATA_CLASS_PREFIX).substringBefore(" (").trim()
                val fields = body.takeWhile { !it.trim().startsWith(")") }
                    .filter { it.trim().startsWith(VAL_PREFIX) }
                    .mapNotNull { parseField(it.trim()) }
                if (fields.isEmpty()) null else DataClassInfo(name, fields)
            }
            .associateBy { it.name }

    fun parseSealedClasses(): Map<String, SealedClassInfo> =
        blocks { it.startsWith(SEALED_CLASS_PREFIX) && it.endsWith("{") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix(SEALED_CLASS_PREFIX).substringBefore(" ").trim()
                val variants = body.takeWhile { it != "}" }.mapNotNull { sealedVariantName(it.trim()) }
                if (variants.isEmpty()) null else SealedClassInfo(name, variants)
            }
            .associateBy { it.name }

    fun parseEnumClasses(): Map<String, EnumClassInfo> =
        blocks { it.startsWith(ENUM_CLASS_PREFIX) && it.contains("{") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix(ENUM_CLASS_PREFIX).substringBefore("(").substringBefore(" {").trim()
                val variants = enumVariants(body)
                if (variants.isEmpty()) null else EnumClassInfo(name, variants)
            }
            .associateBy { it.name }

    // Top-level functions (the UniFFI namespace functions): every `fun` declared at brace-depth 0, i.e.
    // not inside an interface/class/object. The generated source is brace-balanced, so counting `{`/`}`
    // gives the depth before each line reliably.
    fun parseFreeFunctions(): List<MethodSignature> {
        val depthBeforeLine = lines.runningFold(0) { depth, line ->
            depth + line.count { it == '{' } - line.count { it == '}' }
        }
        return lines.mapIndexedNotNull { index, line ->
            if (depthBeforeLine[index] == 0) freeFunctionSignature(line) else null
        }
    }

    // Every top-level declaration whose header line matches [isHeader], paired with the lines after it.
    private fun blocks(isHeader: (String) -> Boolean): List<Decl> =
        lines.withIndex()
            .filter { (_, line) -> isHeader(line) }
            .map { (index, line) -> Decl(line, lines.subList(index + 1, lines.size)) }

    private fun sealedVariantName(line: String): String? = when {
        line.startsWith(DATA_CLASS_PREFIX) -> line.removePrefix(DATA_CLASS_PREFIX).substringBefore("(").trim()
        line.startsWith(OBJECT_PREFIX) -> line.removePrefix(OBJECT_PREFIX).substringBefore(" ").substringBefore(":").trim()
        else -> null
    }

    // Enum entries run from the body start until the first line ending in ';' (inclusive — that line
    // still holds an entry) or the closing '}' (exclusive), whichever comes first; everything after is
    // companion/methods.
    private fun enumVariants(body: List<String>): List<String> {
        val end = body.indexOfFirst { val trimmed = it.trim(); trimmed == "}" || trimmed.endsWith(";") }
        val entries = when {
            end < 0 -> body
            body[end].trim().endsWith(";") -> body.subList(0, end + 1)
            else -> body.subList(0, end)
        }
        return entries.map { it.trim() }
            .filter { it.isNotEmpty() && !it.startsWith("//") }
            .map { it.removeSuffix(",").removeSuffix(";").trim().substringBefore("(").trim() }
            .filter { it.matches(identifierRegex) }
    }

    // Slice a free-function declaration down to the `fun ...`/`suspend fun ...` form
    // [parseMethodSignature] understands, dropping any leading prefix (KDoc `*/`, `@Throws(...)`, etc.)
    // and the trailing ` {` body brace.
    private fun freeFunctionSignature(line: String): MethodSignature? {
        val start = freeFunctionRegex.find(line)?.range?.first ?: return null
        return parseMethodSignature(line.substring(start).substringBefore(" {"))
    }

    private fun parseMethodSignature(line: String): MethodSignature? {
        if (!line.startsWith(FUN_PREFIX) && !line.startsWith(SUSPEND_FUN_PREFIX)) return null
        if (line.contains(COMPANION_OBJECT)) return null

        val isSuspend = line.startsWith(SUSPEND_FUN_PREFIX)
        val withoutPrefix = if (isSuspend) line.removePrefix(SUSPEND_FUN_PREFIX) else line.removePrefix(FUN_PREFIX)

        val name = withoutPrefix.substringBefore("(").removeSurrounding("`")
        val paramsStr = withoutPrefix.substringAfter("(").substringBefore(")")
        val returnType = cleanType(withoutPrefix.substringAfter("): ", "Unit"))

        val params = if (paramsStr.isBlank()) {
            emptyList()
        } else {
            splitParams(paramsStr).mapNotNull { param ->
                val parts = param.split(": ", limit = 2)
                if (parts.size == 2) {
                    Param(parts[0].removeSurrounding("`").trim(), cleanType(parts[1].trim()))
                } else null
            }
        }

        return MethodSignature(name, params, returnType, isSuspend)
    }

    // Split a parameter list on its top-level commas, keeping commas inside generic `<...>` brackets
    // attached to their parameter (e.g. `filter: Map<String, Int>` stays a single parameter).
    private fun splitParams(params: String): List<String> {
        val parts = mutableListOf(StringBuilder())
        var depth = 0
        for (char in params) {
            when (char) {
                '<' -> depth++
                '>' -> depth--
            }
            if (char == ',' && depth == 0) parts.add(StringBuilder()) else parts.last().append(char)
        }
        return parts.map { it.toString().trim() }.filter { it.isNotEmpty() }
    }

    private fun parseField(line: String): Field? {
        val withoutVal = line.removePrefix(VAL_PREFIX).trim()
        val nameRaw = withoutVal.substringBefore(":").removeSurrounding("`").trim()
        val rest = withoutVal.substringAfter(": ", "")
        if (rest.isEmpty()) return null

        val type = rest.substringBefore(" =").trim().removeSuffix(",").trim()
        val default = if (rest.contains(" = ")) {
            rest.substringAfter(" = ").removeSuffix(",").trim()
        } else null

        return Field(nameRaw, cleanType(type), default)
    }

    // Strip the `kotlin.` package prefix from built-in types, e.g. `kotlin.String` -> `String` and
    // `Map<kotlin.String, kotlin.UInt>` -> `Map<String, UInt>`.
    private fun cleanType(type: String): String = type.replace(kotlinPackagePrefix, "")

    // The UniFFI-generated declaration markers this parser keys off of.
    private companion object {
        const val INTERFACE_PREFIX = "public interface "
        const val EXECUTOR_SUFFIX = "RequestExecutorInterface"
        const val DATA_CLASS_PREFIX = "data class "
        const val SEALED_CLASS_PREFIX = "sealed class "
        const val ENUM_CLASS_PREFIX = "enum class "
        const val OBJECT_PREFIX = "object "
        const val VAL_PREFIX = "val "
        const val FUN_PREFIX = "fun "
        const val SUSPEND_FUN_PREFIX = "suspend fun "
        const val COMPANION_OBJECT = "companion object"
    }
}
