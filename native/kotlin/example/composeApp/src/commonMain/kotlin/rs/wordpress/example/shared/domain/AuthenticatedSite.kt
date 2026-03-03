package rs.wordpress.example.shared.domain

import java.net.URL

data class AuthenticatedSite(val id: ULong = 0UL, val name: String, val apiRootUrl: URL)