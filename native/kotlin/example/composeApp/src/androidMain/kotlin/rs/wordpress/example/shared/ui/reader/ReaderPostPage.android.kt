package rs.wordpress.example.shared.ui.reader

import android.annotation.SuppressLint
import android.util.Log
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.Modifier
import androidx.compose.ui.viewinterop.AndroidView
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONObject
import uniffi.wp_mobile.ReaderLink
import uniffi.wp_mobile.readerLink
import uniffi.wp_mobile.readerPageBaseUrl

private const val TAG = "ReaderPostPage"

/**
 * The post's page, in a [WebView].
 *
 * The apps make their web view as tall as the post, inside a scroll view with the rest of the
 * post's screen, using the page's `height:` messages. This one just scrolls.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
actual fun ReaderPostPage(
    html: String,
    postUrl: String?,
    onLink: (link: ReaderLink, url: String) -> Unit,
    onImage: (url: String) -> Unit,
    modifier: Modifier
) {
    val currentPostUrl by rememberUpdatedState(postUrl)
    val currentOnLink by rememberUpdatedState(onLink)
    val currentOnImage by rememberUpdatedState(onImage)

    AndroidView(
        modifier = modifier,
        factory = { context ->
            WebView(context).apply {
                settings.javaScriptEnabled = true
                // Embeds like X's and Instagram's don't work without storage.
                settings.domStorageEnabled = true

                // The shared script posts to `window.readerPost`. Only the page itself can, not
                // the embeds' iframes: the page's origin is its base URL's.
                if (WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER)) {
                    WebViewCompat.addWebMessageListener(
                        this,
                        "readerPost",
                        setOf(readerPageOrigin())
                    ) { view, message, _, isMainFrame, _ ->
                        val data = message.data
                        if (isMainFrame && data != null) {
                            handleMessage(view, data, onImage = { currentOnImage(it) })
                        }
                    }
                } else {
                    Log.w(TAG, "This WebView can't pass the page's messages to the app")
                }

                webViewClient = object : WebViewClient() {
                    override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
                        // Embeds load in iframes, and their links stay in them.
                        if (!request.isForMainFrame) return false
                        // The page only leaves for a link the reader tapped.
                        if (!request.hasGesture()) return true

                        val url = request.url.toString()
                        when (val link = readerLink(url, currentPostUrl)) {
                            is ReaderLink.Anchor -> view.evaluateJavascript(
                                "document.getElementById(${JSONObject.quote(link.id)})?.scrollIntoView()",
                                null
                            )
                            else -> currentOnLink(link, url)
                        }
                        return true
                    }
                }
            }
        },
        update = { webView ->
            if (webView.tag != html) {
                webView.tag = html
                webView.loadDataWithBaseURL(readerPageBaseUrl(), html, "text/html", "utf-8", null)
            }
        }
    )
}

/** `https://wordpress.com`, for `https://wordpress.com/reader`. */
private fun readerPageOrigin(): String {
    val url = android.net.Uri.parse(readerPageBaseUrl())
    return "${url.scheme}://${url.host}"
}

/** The shared script's messages. `reader.js` describes each one. */
private fun handleMessage(webView: WebView, message: String, onImage: (String) -> Unit) {
    // The last field can have colons in it, like a URL, so it's the rest of the message.
    fun fields(count: Int) = message.split(":", limit = count).takeIf { it.size == count }

    when (message.substringBefore(':')) {
        // `anchor:<y>:<id>`. The page is in CSS pixels, as `scrollTo` is.
        "anchor" -> fields(3)?.get(1)?.toDoubleOrNull()?.let { y ->
            webView.evaluateJavascript("window.scrollTo(0, $y)", null)
        }
        // `image:<x>:<y>:<width>:<height>:<url>`
        "image" -> fields(6)?.let { onImage(it[5]) }
        "blocked" -> Log.i(TAG, "The page's Content Security Policy blocked ${message.removePrefix("blocked:")}")
        // `height:`, which this web view doesn't need, and text selection.
        else -> Unit
    }
}
