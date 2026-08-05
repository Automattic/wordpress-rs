// Copy of buildkite-ci's Gradle mirror init script. Keep in sync with it:
// https://github.com/Automattic/buildkite-ci/blob/trunk/src/agents/_shared_/roles/android.install-gradle/files/mirror.gradle.kts
//
// Reposilite dependency mirror — redirects Gradle's dependency repositories
// through our internal Reposilite pull-through cache. Cache misses then egress
// once through the NAT gateway instead of every ephemeral agent re-resolving
// from the upstreams, which is what triggers Maven Central's per-source-IP
// `429 Too Many Requests` throttling.
//
// Two escape hatches, both evaluated up front — Gradle does NOT fall through to
// another repository on transport errors (timeouts/5xx/connection-refused; it
// only does so on a clean 404), so the decision is made before resolution:
//   * REPOSILITE_MIRROR_ENABLED=false  -> kill switch: use the public repos
//   * mirror unreachable               -> automatic fallback to the public repos
//
// REPOSILITE_MIRROR_URL overrides the proxy address without re-provisioning, in
// case the baked-in default below ever changes.

import java.net.HttpURLConnection
import java.net.URI
import org.gradle.api.tasks.testing.Test

val reposilite = System.getenv("REPOSILITE_MIRROR_URL") ?: "http://10.0.2.215:8080"

// Default-ON: only REPOSILITE_MIRROR_ENABLED=false turns the mirror off.
val mirrorEnabled = System.getenv("REPOSILITE_MIRROR_ENABLED")?.lowercase() != "false"

// upstream URL prefix  ->  matching Reposilite repository
val mirrorMap = linkedMapOf(
    "https://repo.maven.apache.org/maven2"     to "$reposilite/maven-central",
    "https://repo1.maven.org/maven2"           to "$reposilite/maven-central",
    "https://dl.google.com/dl/android/maven2"  to "$reposilite/google",
    "https://plugins.gradle.org/m2"            to "$reposilite/gradle-plugins",
)

fun mirrorReachable(): Boolean = runCatching {
    (URI(reposilite).toURL().openConnection() as HttpURLConnection).apply {
        requestMethod = "HEAD"
        connectTimeout = 2000
        readTimeout = 2000
        instanceFollowRedirects = false
        responseCode          // any HTTP response means the server is up
        disconnect()
    }
}.isSuccess                   // any exception (refused / timeout / DNS) -> unreachable

fun RepositoryHandler.redirectToMirror() {
    withType(MavenArtifactRepository::class.java).configureEach {
        val target = mirrorMap.entries.firstOrNull { url.toString().startsWith(it.key) }?.value
        if (target != null) {
            setUrl(target)
            // The proxy is plain HTTP on the private network; Gradle rejects
            // http:// repositories unless this is explicitly opted into.
            isAllowInsecureProtocol = target.startsWith("http://")
        }
    }
}

when {
    !mirrorEnabled ->
        logger.lifecycle("ℹ️  Reposilite mirror disabled via REPOSILITE_MIRROR_ENABLED — using public repositories.")
    !mirrorReachable() ->
        logger.warn("⚠️  Reposilite unreachable at $reposilite — using public repositories.")
    else -> {
        logger.lifecycle("✅  Routing dependencies through Reposilite at $reposilite.")
        gradle.beforeSettings {
            buildscript.repositories.redirectToMirror()
            pluginManagement.repositories.redirectToMirror()
        }
        gradle.settingsEvaluated { dependencyResolutionManagement.repositories.redirectToMirror() }
        gradle.beforeProject     { buildscript.repositories.redirectToMirror() }
        gradle.afterProject      { repositories.redirectToMirror() }

        // Robolectric resolves its `android-all` SDK jars at TEST RUNTIME via its
        // own Maven fetcher, bypassing Gradle's repositories entirely — point it
        // at the mirror too, or it egresses to Maven Central directly and
        // re-triggers the 429s. Harmless for projects that don't use Robolectric.
        gradle.afterProject {
            tasks.withType(Test::class.java).configureEach {
                systemProperty("robolectric.dependency.repo.url", "$reposilite/maven-central")
            }
        }
    }
}
