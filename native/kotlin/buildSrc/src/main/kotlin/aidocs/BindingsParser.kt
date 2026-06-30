package aidocs

// A matched top-level declaration: its header line plus every line that follows it. Callers slice the
// body down to the lines they care about.
private data class Decl(val header: String, val body: List<String>)

// Parses the UniFFI-generated `wp_api.kt` text into a [ParsedBindings] model. Pure: it operates only
// on the lines it is given and performs no I/O.
class BindingsParser(private val lines: List<String>) {

    private val identifierRegex = Regex("^[A-Za-z][A-Za-z0-9_]*$")
    private val kotlinPackagePrefix = Regex("\\bkotlin\\.")

    fun parse(): ParsedBindings = ParsedBindings(
        executors = parseExecutorInterfaces(),
        dataClasses = parseDataClasses(),
        sealedClasses = parseSealedClasses(),
        enumClasses = parseEnumClasses()
    )

    fun parseExecutorInterfaces(): List<ExecutorInterface> =
        blocks { it.startsWith("public interface ") && it.endsWith("RequestExecutorInterface {") }
            .map { (header, body) ->
                val name = header.removePrefix("public interface ").removeSuffix(" {")
                val domain = name.removeSuffix("RequestExecutorInterface").replaceFirstChar { c -> c.lowercase() }
                val methods = body.takeWhile { it != "}" }.mapNotNull { parseMethodSignature(it.trim()) }
                ExecutorInterface(name, domain, methods)
            }

    fun parseDataClasses(): Map<String, DataClassInfo> =
        blocks { it.startsWith("data class ") && it.contains("(") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix("data class ").substringBefore(" (").trim()
                val fields = body.takeWhile { !it.trim().startsWith(")") }
                    .filter { it.trim().startsWith("val ") }
                    .mapNotNull { parseField(it.trim()) }
                if (fields.isEmpty()) null else DataClassInfo(name, fields)
            }
            .associateBy { it.name }

    fun parseSealedClasses(): Map<String, SealedClassInfo> =
        blocks { it.startsWith("sealed class ") && it.endsWith("{") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix("sealed class ").substringBefore(" ").trim()
                val variants = body.takeWhile { it != "}" }.mapNotNull { sealedVariantName(it.trim()) }
                if (variants.isEmpty()) null else SealedClassInfo(name, variants)
            }
            .associateBy { it.name }

    fun parseEnumClasses(): Map<String, EnumClassInfo> =
        blocks { it.startsWith("enum class ") && it.contains("{") }
            .mapNotNull { (header, body) ->
                val name = header.removePrefix("enum class ").substringBefore("(").substringBefore(" {").trim()
                val variants = enumVariants(body)
                if (variants.isEmpty()) null else EnumClassInfo(name, variants)
            }
            .associateBy { it.name }

    // Every top-level declaration whose header line matches [isHeader]. This replaces the four
    // hand-threaded index loops the parsers used to share.
    private fun blocks(isHeader: (String) -> Boolean): List<Decl> =
        lines.withIndex()
            .filter { (_, line) -> isHeader(line) }
            .map { (index, line) -> Decl(line, lines.subList(index + 1, lines.size)) }

    private fun sealedVariantName(line: String): String? = when {
        line.startsWith("data class ") -> line.removePrefix("data class ").substringBefore("(").trim()
        line.startsWith("object ") -> line.removePrefix("object ").substringBefore(" ").substringBefore(":").trim()
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
            splitParams(paramsStr).mapNotNull { param ->
                val parts = param.split(": ", limit = 2)
                if (parts.size == 2) {
                    Param(parts[0].removeSurrounding("`").trim(), parts[1].trim())
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
        val withoutVal = line.removePrefix("val ").trim()
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
}
