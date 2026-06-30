package aidocs

// Structured model of the UniFFI-generated Kotlin bindings. Produced by [BindingsParser] and consumed
// by [DocsGenerator]; pure data, no I/O.

data class Param(val name: String, val type: String)

data class Field(val name: String, val type: String, val default: String?)

data class MethodSignature(
    val name: String,
    val params: List<Param>,
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
    val fields: List<Field>
)

data class SealedClassInfo(
    val name: String,
    val variants: List<String>
)

data class EnumClassInfo(
    val name: String,
    val variants: List<String>
)

/** Everything [BindingsParser] extracts from one bindings file. */
data class ParsedBindings(
    val executors: List<ExecutorInterface>,
    val dataClasses: Map<String, DataClassInfo>,
    val sealedClasses: Map<String, SealedClassInfo>,
    val enumClasses: Map<String, EnumClassInfo>
)
