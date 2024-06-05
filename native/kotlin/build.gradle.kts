plugins {
    alias(libs.plugins.androidApplication) apply false
    alias(libs.plugins.androidLibrary) apply false
    alias(libs.plugins.jetbrainsCompose) apply false
    alias(libs.plugins.compose.compiler) apply false
    alias(libs.plugins.kotlinMultiplatform) apply false
    alias(libs.plugins.kotlinJvm) apply false
    alias(libs.plugins.rustAndroid) apply false
    alias(libs.plugins.publishToS3) apply false

    alias(libs.plugins.detekt)
}

rootProject.ext.set("cargoProjectRoot", "${project.rootDir}/../..")

allprojects {
    apply(plugin = rootProject.libs.plugins.detekt.get().pluginId)

    detekt {
        toolVersion = rootProject.libs.versions.detekt.plugin.get()
        buildUponDefaultConfig = true
        config.from("${project.rootDir}/config/detekt/detekt.yml")
        allRules = false
    }

    tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach {
        jvmTarget = "1.8"
        reports {
            html.required.set(true)
            xml.required.set(true)
        }

        // Exclude generated bindings
        exclude("**/wp_api.kt")
    }

    tasks.withType<io.gitlab.arturbosch.detekt.DetektCreateBaselineTask>().configureEach {
        jvmTarget = "1.8"

        // Exclude generated bindings
        exclude("**/wp_api.kt")
    }

    dependencies {
        detektPlugins(rootProject.libs.detekt.formatting)
    }
}
