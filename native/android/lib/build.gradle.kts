plugins {
    id("org.jetbrains.kotlin.jvm") version "1.8.20"

    `java-library`
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.7.0")
}

testing {
    suites {
        withType(JvmTestSuite::class).matching { it.name in listOf("test", "integrationTest") }.configureEach {
            useKotlinTest("1.8.20")
        }
    }
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

sourceSets {
    main {
        kotlin {
            srcDir("../../../out/uniffi/wordpress_api/")
        }
    }
}
