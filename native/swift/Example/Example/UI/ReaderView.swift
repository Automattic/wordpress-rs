#if canImport(WebKit)
import SwiftUI
import WebKit
import WordPressAPI

/// Shows posts from the WordPress.com Reader the way the apps will: cleaned up and put in a page by
/// `wp_mobile`'s shared Reader rendering, in a web view. It only needs public sites, so it works
/// without signing in.
struct ReaderRootView: View {

    @State
    private var site = "en.blog.wordpress.com"

    @State
    private var path: [ReaderRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            Form {
                Section {
                    TextField("Site address or ID", text: $site)
                        .autocorrectionDisabled()
                        .addressKeyboard()
                        .onSubmit(showPosts)
                    Button("Show Posts", action: showPosts)
                        .disabled(trimmedSite.isEmpty)
                } footer: {
                    Text("A public site, like en.blog.wordpress.com, from the Reader API.")
                }
            }
            .navigationTitle("Reader")
            .navigationDestination(for: ReaderRoute.self) { route in
                switch route {
                case .site(let site):
                    ReaderPostListView(site: site)
                case .post(let site, let postID):
                    ReaderPostView(site: site, postID: postID) { path.append($0) }
                }
            }
        }
    }

    /// The site the Reader API knows by a domain or a site ID, from an address or a domain.
    private var trimmedSite: String {
        let input = site.trimmingCharacters(in: .whitespacesAndNewlines)
        let withoutScheme = input.range(of: "://").map { String(input[$0.upperBound...]) } ?? input
        return String(withoutScheme.prefix { $0 != "/" && $0 != "?" })
    }

    private func showPosts() {
        guard !trimmedSite.isEmpty else { return }
        path.append(.site(trimmedSite))
    }
}

private extension View {
    func addressKeyboard() -> some View {
        #if os(iOS)
        self.textInputAutocapitalization(.never).keyboardType(.URL)
        #else
        self
        #endif
    }
}

enum ReaderRoute: Hashable {
    case site(String)
    case post(site: String, postID: UInt64)
}

// MARK: - Posts

struct ReaderPostListView: View {

    let site: String

    @State
    private var posts: [ReaderPostSummary]?

    @State
    private var error: Error?

    var body: some View {
        Group {
            if let posts {
                List(posts) { post in
                    NavigationLink(value: ReaderRoute.post(site: site, postID: post.id)) {
                        VStack(alignment: .leading) {
                            Text(post.displayTitle)
                            Text(post.date.prefix(10))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                }
            } else if let error {
                ContentUnavailableView(
                    "Couldn't load the posts",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error.localizedDescription)
                )
            } else {
                ProgressView()
            }
        }
        .navigationTitle(site)
        .task {
            do {
                posts = try await ReaderAPI.posts(site: site)
            } catch {
                self.error = error
            }
        }
    }
}

// MARK: - A post

struct ReaderPostView: View {

    let site: String
    let postID: UInt64
    let open: (ReaderRoute) -> Void

    @AppStorage("readerTheme")
    private var themeID = ReaderTheme.system.id()

    @AppStorage("readerFont")
    private var fontName = Self.fonts[0].name

    @AppStorage("readerTextScale")
    private var textScale = 1.0

    @State
    private var post: ReaderAPIPost?

    @State
    private var content: ReaderPostContent?

    @State
    private var error: Error?

    @State
    private var viewedImage: ViewedImage?

    @Environment(\.openURL)
    private var openURL

    var body: some View {
        Group {
            if let post, let content {
                ReaderWebView(
                    html: content.pageHtml(options: pageOptions),
                    postURL: post.url,
                    onLink: handle,
                    onImage: { viewedImage = ViewedImage(url: $0) }
                )
                .ignoresSafeArea(edges: .bottom)
            } else if let error {
                ContentUnavailableView(
                    "Couldn't load the post",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error.localizedDescription)
                )
            } else {
                ProgressView()
            }
        }
        .navigationTitle(post?.displayTitle ?? "")
        .toolbar {
            Menu {
                Picker("Theme", selection: $themeID) {
                    ForEach(readerThemes(), id: \.self) { theme in
                        Text(theme.description).tag(theme.id())
                    }
                }
                Picker("Font", selection: $fontName) {
                    ForEach(Self.fonts, id: \.name) { font in
                        Text(font.name).tag(font.name)
                    }
                }
                Picker("Text Size", selection: $textScale) {
                    ForEach(Self.textScales, id: \.self) { scale in
                        Text(scale.formatted(.percent)).tag(scale)
                    }
                }
            } label: {
                Label("Reading Settings", systemImage: "textformat.size")
            }
        }
        .sheet(item: $viewedImage) { image in
            ReaderImageView(url: image.url)
        }
        .task {
            do {
                let post = try await ReaderAPI.post(site: site, postID: postID)
                let content = ReaderPostContent(html: post.content, url: post.url)
                // Photon, which makes the smaller image sizes, can't fetch a private site's images.
                self.content = post.siteIsPrivate == true ? content : content.withPhotonSizes()
                self.post = post
            } catch {
                self.error = error
            }
        }
    }

    private var pageOptions: ReaderPageOptions {
        ReaderPageOptions(
            theme: readerThemeFromId(id: themeID) ?? .system,
            appColors: Self.appColors,
            font: Self.fonts.first { $0.name == fontName }?.font,
            textScale: textScale,
            // The web view lays the post out with margins, and draws the theme's background around it.
            css: "html { background-color: var(--reader-background); } body { padding: 16px; }"
        )
    }

    private func handle(_ link: ReaderLink, url: URL) {
        switch link {
        case .sitePost(let siteID, let postID):
            open(.post(site: String(siteID), postID: postID))
        case .image(let imageURL):
            viewedImage = URL(string: imageURL).map(ViewedImage.init)
        default:
            // A real app opens the rest in the Reader, a player, or the image viewer. Here, the
            // system opens them.
            openURL(url)
        }
    }

    private struct ViewedImage: Identifiable {
        let url: URL
        var id: URL { url }
    }

    private static let fonts: [(font: ReaderFont, name: String)] = [(.sans, "Sans"), (.serif, "Serif"), (.mono, "Mono")]

    private static let textScales: [Double] = [0.75, 0.9, 1.0, 1.15, 1.25, 1.4, 1.6]

    /// The app's own colours, for the System theme: the platform's, with the WordPress app's colour
    /// for links.
    private static var appColors: ReaderAppColors {
        let defaults = readerDefaultAppColors(brand: .wordpress)
        func colors(_ scheme: ColorScheme, defaults: ReaderThemeColors) -> ReaderThemeColors {
            var environment = EnvironmentValues()
            environment.colorScheme = scheme
            func color(_ color: Color, or fallback: ReaderColor) -> ReaderColor {
                ReaderColor(color, in: environment) ?? fallback
            }
            return ReaderThemeColors(
                text: color(PlatformColors.text, or: defaults.text),
                secondaryText: color(PlatformColors.secondaryText, or: defaults.secondaryText),
                tertiaryText: color(PlatformColors.tertiaryText, or: defaults.tertiaryText),
                background: color(PlatformColors.background, or: defaults.background),
                secondaryBackground: color(PlatformColors.secondaryBackground, or: defaults.secondaryBackground),
                border: color(PlatformColors.border, or: defaults.border),
                link: defaults.link
            )
        }
        return ReaderAppColors(
            light: colors(.light, defaults: defaults.light),
            dark: colors(.dark, defaults: defaults.dark)
        )
    }
}

/// The platform's colours for each of the page's, which look different in light and dark mode.
private enum PlatformColors {
    #if canImport(UIKit)
    static let text = Color(uiColor: .label)
    static let secondaryText = Color(uiColor: .secondaryLabel)
    static let tertiaryText = Color(uiColor: .tertiaryLabel)
    static let background = Color(uiColor: .systemBackground)
    static let secondaryBackground = Color(uiColor: .secondarySystemBackground)
    static let border = Color(uiColor: .separator)
    #else
    static let text = Color(nsColor: .textColor)
    static let secondaryText = Color(nsColor: .secondaryLabelColor)
    static let tertiaryText = Color(nsColor: .tertiaryLabelColor)
    static let background = Color(nsColor: .textBackgroundColor)
    static let secondaryBackground = Color(nsColor: .windowBackgroundColor)
    static let border = Color(nsColor: .separatorColor)
    #endif
}

private struct ReaderImageView: View {

    let url: URL

    @Environment(\.dismiss)
    private var dismiss

    var body: some View {
        NavigationStack {
            AsyncImage(url: url) { image in
                image.resizable().scaledToFit()
            } placeholder: {
                ProgressView()
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .toolbar {
                Button("Done") { dismiss() }
            }
        }
    }
}

// MARK: - The web view

/// The web view for a Reader post page. It handles the shared script's messages, and the post's links.
///
/// The apps make their web view as tall as the post, inside a scroll view with the rest of the
/// post's screen, using the page's `height:` messages. This one just scrolls.
@MainActor
struct ReaderWebView {

    let html: String
    let postURL: String?
    let onLink: (ReaderLink, URL) -> Void
    let onImage: (URL) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(parent: self)
    }

    private func makeWebView(coordinator: Coordinator) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.userContentController.add(coordinator, name: "readerPost")
        let webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = coordinator
        return webView
    }

    private func update(_ webView: WKWebView, coordinator: Coordinator) {
        coordinator.parent = self
        guard coordinator.loadedHTML != html else { return }
        coordinator.loadedHTML = html
        webView.loadHTMLString(html, baseURL: URL(string: readerPageBaseUrl()))
    }

    private static func dismantle(_ webView: WKWebView) {
        webView.configuration.userContentController.removeScriptMessageHandler(forName: "readerPost")
    }

    final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {

        var parent: ReaderWebView
        var loadedHTML: String?

        init(parent: ReaderWebView) {
            self.parent = parent
        }

        /// The shared script's messages. `reader.js` describes each one.
        func userContentController(
            _ userContentController: WKUserContentController,
            didReceive message: WKScriptMessage
        ) {
            guard message.frameInfo.isMainFrame, let body = message.body as? String else { return }
            // The last field can have colons in it, like a URL, so it's the rest of the message.
            func fields(_ count: Int) -> [Substring]? {
                let fields = body.split(separator: ":", maxSplits: count - 1, omittingEmptySubsequences: false)
                return fields.count == count ? fields : nil
            }
            switch body.prefix(while: { $0 != ":" }) {
            case "anchor":
                // `anchor:<y>:<id>`
                if let fields = fields(3), let offset = Double(fields[1]), let webView = message.webView {
                    webView.callAsyncJavaScript(
                        "window.scrollTo(0, offset)",
                        arguments: ["offset": offset],
                        in: nil,
                        in: .page,
                        completionHandler: nil
                    )
                }
            case "image":
                // `image:<x>:<y>:<width>:<height>:<url>`
                if let fields = fields(6), let url = URL(string: String(fields[5])) {
                    parent.onImage(url)
                }
            case "blocked":
                debugPrint("The Reader page's Content Security Policy blocked \(body.dropFirst("blocked:".count))")
            default:
                // `height:`, which this web view doesn't need, and text selection.
                break
            }
        }

        func webView(
            _ webView: WKWebView,
            decidePolicyFor navigationAction: WKNavigationAction
        ) async -> WKNavigationActionPolicy {
            // Embeds load in iframes, and their links stay in them.
            let isMainFrame = navigationAction.targetFrame?.isMainFrame ?? true
            guard isMainFrame else {
                return .allow
            }
            // The page only leaves for a link the reader tapped.
            guard navigationAction.navigationType == .linkActivated, let url = navigationAction.request.url else {
                return navigationAction.request.url == URL(string: readerPageBaseUrl()) ? .allow : .cancel
            }

            let link = readerLink(url: url.absoluteString, postUrl: parent.postURL)
            if case .anchor(let id) = link {
                webView.callAsyncJavaScript(
                    "document.getElementById(id)?.scrollIntoView()",
                    arguments: ["id": id],
                    in: nil,
                    in: .page,
                    completionHandler: nil
                )
            } else {
                parent.onLink(link, url)
            }
            return .cancel
        }
    }
}

#if os(macOS)
extension ReaderWebView: NSViewRepresentable {
    func makeNSView(context: Context) -> WKWebView {
        makeWebView(coordinator: context.coordinator)
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator)
    }

    static func dismantleNSView(_ webView: WKWebView, coordinator: Coordinator) {
        dismantle(webView)
    }
}
#else
extension ReaderWebView: UIViewRepresentable {
    func makeUIView(context: Context) -> WKWebView {
        makeWebView(coordinator: context.coordinator)
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator)
    }

    static func dismantleUIView(_ webView: WKWebView, coordinator: Coordinator) {
        dismantle(webView)
    }
}
#endif

// MARK: - The Reader API

/// A post in a list of a site's posts.
struct ReaderPostSummary: Decodable, Identifiable {
    let id: UInt64
    let title: String
    let date: String

    /// The title's text: the API sends HTML, like `&#8217;` for an apostrophe.
    var displayTitle: String {
        ReaderPostContent(html: title).plainText()
    }

    enum CodingKeys: String, CodingKey {
        case id = "ID", title, date
    }
}

/// A post, with its content.
struct ReaderAPIPost: Decodable {
    let title: String
    let url: String
    let content: String
    let siteIsPrivate: Bool?

    var displayTitle: String {
        ReaderPostContent(html: title).plainText()
    }

    enum CodingKeys: String, CodingKey {
        case title, content, url = "URL", siteIsPrivate = "site_is_private"
    }
}

/// The public Reader API, which the Reader shows posts from. Public sites' posts don't need signing in.
enum ReaderAPI {

    private struct ErrorMessage: Decodable {
        let message: String?
    }

    struct Failure: LocalizedError {
        let status: Int
        let message: String?

        var errorDescription: String? {
            "The Reader API answered \(status): \(message ?? "no message")"
        }
    }

    static func posts(site: String) async throws -> [ReaderPostSummary] {
        struct Posts: Decodable {
            let posts: [ReaderPostSummary]
        }
        let query = [
            URLQueryItem(name: "number", value: "30"),
            URLQueryItem(name: "fields", value: "ID,title,date")
        ]
        return try await get(Posts.self, path: [site, "posts"], query: query).posts
    }

    static func post(site: String, postID: UInt64) async throws -> ReaderAPIPost {
        try await get(ReaderAPIPost.self, path: [site, "posts", String(postID)], query: [])
    }

    private static func get<T: Decodable>(_ type: T.Type, path: [String], query: [URLQueryItem]) async throws -> T {
        // swiftlint:disable:next force_unwrapping
        var url = URL(string: "https://public-api.wordpress.com/rest/v1.2/read/sites")!
        for component in path {
            url.append(path: component)
        }
        url.append(queryItems: query)

        let (data, response) = try await URLSession.shared.data(from: url)
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard (200..<300).contains(status) else {
            throw Failure(status: status, message: try? JSONDecoder().decode(ErrorMessage.self, from: data).message)
        }
        return try JSONDecoder().decode(type, from: data)
    }
}
#endif
