rootProject.name = "wordpress-rs"
enableFeaturePreview("TYPESAFE_PROJECT_ACCESSORS")

pluginManagement {
    repositories {
        maven {
            url = uri("https://a8c-libs.s3.amazonaws.com/android")
            content {
                includeGroup("com.automattic.android")
                includeGroup("com.automattic.android.publish-to-s3")
            }
        }
        gradlePluginPortal()
        google()
    }
}

include(":api:kotlin")
include(":api:android")
