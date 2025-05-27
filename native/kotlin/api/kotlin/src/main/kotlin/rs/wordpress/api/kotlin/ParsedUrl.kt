package rs.wordpress.api.kotlin

import uniffi.wp_api.ParsedUrl
import java.net.URL

operator fun ParsedUrl.Companion.invoke(url: URL) = parse(url.toString())

fun ParsedUrl.toURL(): URL = URL(this.url())
