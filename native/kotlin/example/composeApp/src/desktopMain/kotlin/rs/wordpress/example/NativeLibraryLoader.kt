package rs.wordpress.example

import java.io.File
import java.nio.file.Files

object NativeLibraryLoader {
    private val tempDir = Files.createTempDirectory("wordpress-rs-libs").toFile().apply {
        deleteOnExit()
    }

    fun loadLibraries() {
        // Determine the library name based on the OS
        val osName = System.getProperty("os.name").lowercase()
        val moduleName = BuildConfig.RUST_PRIMARY_MODULE
        val libName = when {
            osName.contains("mac") || osName.contains("darwin") -> "lib${moduleName}.dylib"
            osName.contains("linux") -> "lib${moduleName}.so"
            osName.contains("windows") -> "${moduleName}.dll"
            else -> throw UnsupportedOperationException("Unsupported OS: $osName")
        }

        // Extract the native library from jar
        try {
            val resourceStream = javaClass.classLoader.getResourceAsStream(libName)
            if (resourceStream != null) {
                val tempFile = File(tempDir, libName).apply {
                    deleteOnExit()
                }

                resourceStream.use { input ->
                    tempFile.outputStream().use { output ->
                        input.copyTo(output)
                    }
                }

                // Make executable
                tempFile.setExecutable(true)
                println("Extracted native library: ${tempFile.absolutePath}")
            } else {
                println("Warning: Could not find $libName in resources")
            }
        } catch (e: Exception) {
            println("Warning: Could not extract $libName: ${e.message}")
        }

        // Set JNA library path to our temp directory
        System.setProperty("jna.library.path", tempDir.absolutePath)
        System.setProperty("java.library.path", tempDir.absolutePath)

        println("Native library path set to: ${tempDir.absolutePath}")
    }
}
