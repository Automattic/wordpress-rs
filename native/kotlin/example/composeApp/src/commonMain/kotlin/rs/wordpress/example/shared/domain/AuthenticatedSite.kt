package rs.wordpress.example.shared.domain

import uniffi.wp_api.ParsedUrl

data class AuthenticatedSite(val name: String, val url: ParsedUrl)