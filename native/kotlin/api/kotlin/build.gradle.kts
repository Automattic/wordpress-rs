plugins {
    alias(libs.plugins.kotlinJvm)
    alias(libs.plugins.kotlinSerialization)
    alias(libs.plugins.publishToS3)
    id("java-library")
    id("jvm-test-suite")
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

@Suppress("UnstableApiUsage")
testing {
    suites {
        val test by getting(JvmTestSuite::class) {
            useJUnit(rootProject.libs.versions.junit.get())
        }

        register<JvmTestSuite>("integrationTest") {
            testType = TestSuiteType.INTEGRATION_TEST

            sources {
                resources {
                    setSrcDirs(
                        listOf(
                            rootProject.ext.get("jniLibsPath"),
                            rootProject.ext.get("generatedTestResourcesPath")
                        )
                    )
                }
            }

            dependencies {
                implementation(project())

                implementation(rootProject.libs.kotlin.test)
                implementation(rootProject.libs.kotlinx.coroutines.test)
                implementation(libs.kotlinx.serialization)
            }

            targets {
                all {
                    testTask.configure {
                        shouldRunAfter(test)
                    }
                }
            }
        }
    }
}

tasks.withType<Test>().configureEach {
    afterTest(KotlinClosure2({ descriptor: TestDescriptor, result: TestResult ->
        println("[${descriptor.className}] > ${descriptor.displayName}: ${result.resultType}")
    }))
}

@Suppress("UnstableApiUsage")
tasks.named("check") {
    dependsOn(testing.suites.named("integrationTest"))
}

dependencies {
    implementation(libs.okhttp)
    implementation(libs.jna)
    implementation(libs.kotlinx.coroutines.core)
    if (project.hasProperty("wpApiBindingsVersion")) {
        api("rs.wordpress.api:bindings:${project.properties["wpApiBindingsVersion"]}") {
            exclude(group = "net.java.dev.jna")
        }
    } else {
        api(project(":api:bindings")) {
            exclude(group = "net.java.dev.jna")
        }
    }
}

tasks.named("processIntegrationTestResources").configure {
    dependsOn(rootProject.tasks.named("copyDesktopJniLibs"))
    dependsOn(rootProject.tasks.named("copyTestCredentials"))
}

project.afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("maven") {
                from(components["java"])

                groupId = "rs.wordpress.api"
                artifactId = "kotlin"
                // version is set by "publish-to-s3" plugin
            }
        }
    }
}
