package rs.wordpress.example.shared

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform