package rs.wordpress.example.shared

class JVMPlatform: Platform {
    override val name: String = "Java ${System.getProperty("java.version")}"
}

actual fun getPlatform(): Platform = JVMPlatform()

actual fun localTestSiteUrl() = object: TestSiteUrl {
    override val siteUrl: String
        get() = "http://localhost"
}