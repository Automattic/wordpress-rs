import SwiftUI

#if canImport(WebKit)
import WebKit

/// A post's HTML, as a web page. Scripts don't run, and a link opens in the browser rather than in place.
struct ReaderHTMLView {
    let html: String
    /// The post's address, which its relative links and images are read against.
    let baseURL: URL?

    final class Coordinator: NSObject, WKNavigationDelegate {
        var loadedHTML: String?
        var openURL: OpenURLAction?

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction
        ) async -> WKNavigationActionPolicy {
            guard navigationAction.navigationType == .linkActivated else {
                return .allow
            }
            if let url = navigationAction.request.url {
                openURL?(url)
            }
            return .cancel
        }
    }

    @MainActor
    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    @MainActor
    fileprivate func makeWebView(coordinator: Coordinator) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = coordinator
        return webView
    }

    @MainActor
    fileprivate func update(_ webView: WKWebView, coordinator: Coordinator, openURL: OpenURLAction) {
        coordinator.openURL = openURL
        guard coordinator.loadedHTML != html else { return }
        coordinator.loadedHTML = html
        webView.loadHTMLString(Self.page(body: html), baseURL: baseURL)
    }

    private static func page(body: String) -> String {
        """
        <!doctype html>
        <html>
        <head>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
        :root { color-scheme: light dark; }
        body { font: -apple-system-body; margin: 16px; overflow-wrap: break-word; }
        img, video, iframe { max-width: 100%; height: auto; }
        figure { margin: 0; }
        </style>
        </head>
        <body>\(body)</body>
        </html>
        """
    }
}

#if os(macOS)
extension ReaderHTMLView: NSViewRepresentable {
    func makeNSView(context: Context) -> WKWebView {
        makeWebView(coordinator: context.coordinator)
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator, openURL: context.environment.openURL)
    }
}
#else
extension ReaderHTMLView: UIViewRepresentable {
    func makeUIView(context: Context) -> WKWebView {
        makeWebView(coordinator: context.coordinator)
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator, openURL: context.environment.openURL)
    }
}
#endif

#else

/// A post's HTML, as plain text: this platform has no web view.
struct ReaderHTMLView: View {
    let html: String
    let baseURL: URL?

    var body: some View {
        ScrollView {
            Text(html.readerPlainText)
                .padding()
        }
    }
}

#endif
