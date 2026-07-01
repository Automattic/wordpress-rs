package aidocs

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

// Inputs mirror the real UniFFI-generated `wp_api.kt` formatting: backtick-quoted names, commas on
// their own line, `){` data-class terminators, and `;`-terminated enum entries followed by methods.
class BindingsParserTest {

    @Test
    fun `parses executor interface domain and method signatures`() {
        val lines = listOf(
            "public interface PostsRequestExecutorInterface {",
            "    ",
            "    fun `cancel`(`context`: RequestContext)",
            "    ",
            "    suspend fun `fetchAuthenticationState`(): AuthenticationState?",
            "    ",
            "    suspend fun `list`(`params`: PostListParams): PostListResponse",
            "}"
        )

        val interfaces = BindingsParser(lines).parseExecutorInterfaces()

        assertEquals(1, interfaces.size)
        val executor = interfaces.single()
        assertEquals("PostsRequestExecutorInterface", executor.name)
        // domain strips the suffix and lowercases the first char.
        assertEquals("posts", executor.domain)
        // The parser keeps every method; filtering `cancel`/`fetchAuthenticationState` is the generator's job.
        assertEquals(
            listOf(
                MethodSignature("cancel", listOf(Param("context", "RequestContext")), "Unit", isSuspend = false),
                MethodSignature("fetchAuthenticationState", emptyList(), "AuthenticationState?", isSuspend = true),
                MethodSignature("list", listOf(Param("params", "PostListParams")), "PostListResponse", isSuspend = true)
            ),
            executor.methods
        )
    }

    @Test
    fun `parses data class fields with defaults and cleaned types`() {
        val lines = listOf(
            "data class PostListParams (",
            "    val `search`: kotlin.String? = null",
            "    , ",
            "    val `page`: kotlin.UInt",
            "    ",
            "){"
        )

        val dataClasses = BindingsParser(lines).parseDataClasses()

        val params = dataClasses["PostListParams"]
        assertEquals(
            listOf(
                Field("search", "String?", "null"),
                Field("page", "UInt", null)
            ),
            params?.fields
        )
    }

    @Test
    fun `parses sealed class data-class and object variants`() {
        val lines = listOf(
            "sealed class ApplicationPasswordsNotSupportedReason {",
            "    ",
            "    data class ApplicationPasswordBlockedByPlugin(",
            "        val `plugin`: uniffi.wp_api.KnownAuthenticationBlockingPlugin) : ApplicationPasswordsNotSupportedReason()",
            "    {",
            "        companion object",
            "    }",
            "    ",
            "    object ApplicationPasswordBlockedByMultiplePlugins : ApplicationPasswordsNotSupportedReason()",
            "}"
        )

        val sealedClasses = BindingsParser(lines).parseSealedClasses()

        assertEquals(
            listOf("ApplicationPasswordBlockedByPlugin", "ApplicationPasswordBlockedByMultiplePlugins"),
            sealedClasses["ApplicationPasswordsNotSupportedReason"]?.variants
        )
    }

    @Test
    fun `parses enum entries terminated by semicolon and skips trailing methods and comments`() {
        val lines = listOf(
            "enum class AuthenticationState {",
            "    ",
            "    // backed by the Rust enum",
            "    AUTHENTICATED,",
            "    UNAUTHORIZED;",
            "",
            "    companion object",
            "}"
        )

        val enums = BindingsParser(lines).parseEnumClasses()

        // The `;` ends the entry list, so `companion object` is not captured as a variant.
        assertEquals(
            listOf("AUTHENTICATED", "UNAUTHORIZED"),
            enums["AuthenticationState"]?.variants
        )
    }

    @Test
    fun `parses enum terminated by a brace with no trailing semicolon`() {
        val lines = listOf(
            "enum class SimpleKind {",
            "    FIRST,",
            "    SECOND",
            "}"
        )

        val enums = BindingsParser(lines).parseEnumClasses()

        // No `;`: the entry list runs until the closing `}` instead.
        assertEquals(
            listOf("FIRST", "SECOND"),
            enums["SimpleKind"]?.variants
        )
    }

    @Test
    fun `parse bundles all four model kinds`() {
        val lines = listOf(
            "public interface MeRequestExecutorInterface {",
            "    suspend fun `get`(): MeResponse",
            "}",
            "data class MeParams (",
            "    val `context`: kotlin.String",
            "){",
            "sealed class MeError {",
            "    object NotFound : MeError()",
            "}",
            "enum class MeKind {",
            "    A,",
            "    B;",
            "}"
        )

        val parsed = BindingsParser(lines).parse()

        assertEquals(listOf("me"), parsed.executors.map { it.domain })
        assertTrue(parsed.dataClasses.containsKey("MeParams"))
        assertEquals(listOf("NotFound"), parsed.sealedClasses["MeError"]?.variants)
        assertEquals(listOf("A", "B"), parsed.enumClasses["MeKind"]?.variants)
        // The indented `object` variant inside the sealed class is not double-counted as a data class.
        assertNull(parsed.dataClasses["MeError"])
    }

    @Test
    fun `keeps a comma'd generic param as a single parameter`() {
        val lines = listOf(
            "public interface FilterRequestExecutorInterface {",
            "    suspend fun `query`(`filter`: Map<kotlin.String, kotlin.Int>): FilterResponse",
            "}"
        )

        val method = BindingsParser(lines).parseExecutorInterfaces().single().methods.single()

        assertEquals(listOf(Param("filter", "Map<String, Int>")), method.params)
    }

    @Test
    fun `parses top-level free functions and ignores interface methods`() {
        val lines = listOf(
            "public interface PostsRequestExecutorInterface {",
            "    suspend fun `list`(`params`: PostListParams): PostListResponse",
            "}",
            " fun `setupLogger`(`appId`: kotlin.String)",
            "        =",
            "    uniffiRustCall() { _status -> }",
            "    @Throws(OAuthResponseUrlException::class) fun `extractLoginDetailsFromUrl`(`url`: kotlin.String): WpApiApplicationPasswordDetails {",
            "        return foo",
            "    }",
            " suspend fun `fetchAuthenticationState`(): AuthenticationState? {",
            "        return bar",
            "    }",
            "    /**",
            "     * A documented function.",
            "     */ fun `buildTokenRequestUrl`(`state`: kotlin.String): ParsedUrl {",
            "        return baz",
            "    }"
        )

        val functions = BindingsParser(lines).parseFreeFunctions()

        // `list` is a method (brace-depth 1) and is excluded; the top-level functions are kept regardless
        // of leading `@Throws`/KDoc prefixes or the `suspend` modifier.
        assertEquals(
            listOf(
                MethodSignature("setupLogger", listOf(Param("appId", "String")), "Unit", isSuspend = false),
                MethodSignature(
                    "extractLoginDetailsFromUrl",
                    listOf(Param("url", "String")),
                    "WpApiApplicationPasswordDetails",
                    isSuspend = false
                ),
                MethodSignature("fetchAuthenticationState", emptyList(), "AuthenticationState?", isSuspend = true),
                MethodSignature("buildTokenRequestUrl", listOf(Param("state", "String")), "ParsedUrl", isSuspend = false)
            ),
            functions
        )
    }
}
