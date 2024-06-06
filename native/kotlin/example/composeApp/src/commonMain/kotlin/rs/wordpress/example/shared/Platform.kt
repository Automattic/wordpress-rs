package rs.wordpress.example.shared

import rs.wordpress.api.kotlin.WpApiClient

interface Platform {
    val name: String
}

expect fun getPlatform(): Platform

expect fun createWpApiClient(): WpApiClient