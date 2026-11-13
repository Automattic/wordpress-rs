package rs.wordpress.api.kotlin

import uniffi.wp_api.ParsedUrl
import java.net.URI
import java.net.URL

operator fun ParsedUrl.Companion.invoke(url: URL) = parse(url.toString())

fun ParsedUrl.toURL(): URL = URI(this.url()).toURL()
