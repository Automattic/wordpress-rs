package rs.wordpress.api.kotlin

import org.junit.jupiter.api.Test
import kotlin.test.assertTrue

/**
 * Guards the `localizedDescription` codegen (see `build.gradle.kts` →
 * `appendLocalizedErrorExtensions`).
 *
 * The `WpDeriveLocalizable` macro emits one `localize<Type>` free function per Rust
 * `WpSupportsLocalization` impl; the binding post-process appends a `localizedDescription`
 * extension for each, at parity with Swift's generated `LocalizedError.errorDescription`.
 * This fails if the post-process ever misses a localizer — e.g. a new type whose generated
 * signature the regex doesn't match — so the gap can't silently reopen.
 */
class LocalizedErrorParityTest {

    @Test
    fun everyLocalizableTypeHasAnExtension() {
        val methods = GENERATED_BINDING_FILE_CLASSES
            .flatMap { Class.forName(it).declaredMethods.asList() }

        // `localize<Type>(value: <Type>, locale: WpLocale?)` — receiver is param 0.
        // `localizedDescription` also starts with "localize", so match `localize<Uppercase>`.
        val localizable = methods
            .filter { it.name.matches(Regex("localize[A-Z].*")) }
            .map { it.parameterTypes[0] }
            .toSet()

        // `T.localizedDescription(locales)` compiles to `localizedDescription(T, List)`;
        // its `$default` overload is a distinct method name, so it's excluded here.
        val extended = methods
            .filter { it.name == "localizedDescription" }
            .map { it.parameterTypes[0] }
            .toSet()

        val missing = localizable - extended
        val extra = extended - localizable
        assertTrue(
            missing.isEmpty() && extra.isEmpty(),
            "localizedDescription drifted from the WpSupportsLocalization set.\n" +
                "  Missing a generated extension for: ${missing.map { it.simpleName }}\n" +
                "  Extension without a matching localizer: ${extra.map { it.simpleName }}",
        )
    }

    private companion object {
        // JVM file-facade classes holding the generated `localize<Type>` functions and,
        // after post-processing, their `localizedDescription` extensions.
        val GENERATED_BINDING_FILE_CLASSES = listOf(
            "uniffi.wp_api.Wp_apiKt",
            "uniffi.wp_mobile.Wp_mobileKt",
        )
    }
}
