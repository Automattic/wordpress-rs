package rs.wordpress.example.shared

interface TestSiteUrl {
    val siteUrl: String
}

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform

expect fun localTestSiteUrl(): TestSiteUrl