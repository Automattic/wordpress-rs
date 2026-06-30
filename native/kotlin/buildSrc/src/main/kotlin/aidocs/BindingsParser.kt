package aidocs

// Parses the UniFFI-generated `wp_api.kt` text into a [ParsedBindings] model. Pure: it operates only
// on the lines it is given and performs no I/O.
class BindingsParser(private val lines: List<String>) {

    private val identifierRegex = Regex("^[A-Za-z][A-Za-z0-9_]*$")

    fun parse(): ParsedBindings = ParsedBindings(
        executors = parseExecutorInterfaces(),
        dataClasses = parseDataClasses(),
        sealedClasses = parseSealedClasses(),
        enumClasses = parseEnumClasses()
    )

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
                val fields = mutableListOf<Field>()
                i++

                while (i < lines.size) {
                    val fieldLine = lines[i].trim()
                    if (fieldLine.startsWith("val ")) {
                        val parsed = parseField(fieldLine)
                        if (parsed != null) fields.add(parsed)
                    } else if (fieldLine.startsWith(")")) {
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
                    Param(parts[0].removeSurrounding("`").trim(), parts[1].trim())
                } else null
            }
        }

        return MethodSignature(name, params, returnType, isSuspend)
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
