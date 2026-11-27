import SwiftUI
import WordPressAPI
import WordPressApiCache

#if canImport(WebKit)
import WebKit
#endif

/// Where the example app keeps saved posts' images, and how much room they may take.
enum ReaderOfflineImages {
    /// Enough for a few dozen posts, and small enough that running out of room can be seen.
    static let maxBytes: UInt64 = 50_000_000

    /// The scheme a stored image is shown through: `reader-offline://image/<file name>`.
    static let scheme = "reader-offline"
    static let urlPrefix = "\(scheme)://image/"

    static func directory() throws -> URL {
        try FileManager.default
            .url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            .appendingPathComponent("reader-offline-images", isDirectory: true)
    }
}

extension ReaderOfflineState {
    /// The state in words.
    var label: String {
        switch self {
        case .notDownloaded: "Not downloaded"
        case .downloading: "Downloading…"
        // A post with no images to store is whole too, so "with images" would say too much.
        case .ready(let hasImages): hasImages ? "Ready" : "Ready, text only"
        case .failed(let reason): "Failed: \(reason)"
        }
    }

    var isReady: Bool {
        if case .ready = self { true } else { false }
    }

    var isFailed: Bool {
        if case .failed = self { true } else { false }
    }
}

struct ReaderOfflineRow: Identifiable {
    let id: String
    let key: ReaderSavedKey
    let title: String
    let detail: String
    let state: ReaderOfflineState

    init(_ item: ReaderSavedListItem) {
        id = item.key.rowID
        key = item.key
        title = item.post?.displayTitle ?? item.key.rowID
        detail = [
            item.post?.siteName,
            item.dateSaved.map { "Saved \($0.readerLabel)" },
            item.post?.date.map { "Published \($0.readerLabel)" }
        ]
        .compactMap { $0 }
        .joined(separator: " · ")
        state = item.offline
    }
}

/// How the saved list is ordered on the offline screen.
enum ReaderOfflineOrder: String, CaseIterable, Identifiable {
    case newestSave = "Newest save first"
    case oldestSave = "Oldest save first"
    case newestPublished = "Newest post first"
    case oldestPublished = "Oldest post first"

    var id: String { rawValue }

    var orderBy: ReaderSavedOrderBy {
        switch self {
        case .newestSave, .oldestSave: .date
        case .newestPublished, .oldestPublished: .datePublished
        }
    }

    var order: ReaderStreamOrder {
        switch self {
        case .newestSave, .newestPublished: .desc
        case .oldestSave, .oldestPublished: .asc
        }
    }
}

/// The saved posts and what the device has of each for reading with no connection.
@MainActor
@Observable
final class ReaderOfflineViewModel {
    let session: ReaderSession

    @ObservationIgnored private let saved: ReaderSavedService
    @ObservationIgnored private var collection: ReaderSavedPostsCollection
    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false

    private(set) var rows: [ReaderOfflineRow] = []
    private(set) var imageBytes: UInt64 = 0
    private(set) var report: String?
    private(set) var isWorking = false

    var order = ReaderOfflineOrder.newestSave {
        didSet {
            guard order != oldValue else { return }
            collection = saved.savedPostsOrdered(orderBy: order.orderBy, order: order.order)
            Task { await reloadFromCache() }
        }
    }

    /// What went wrong, in words, shown as an alert.
    var errorMessage: String?

    init(session: ReaderSession) {
        self.session = session
        saved = session.service.saved()
        collection = saved.savedPosts()
    }

    /// Shows what the cache has. Nothing is fetched until the reader asks.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        // Any change to the saved posts or to what's stored of them reloads the rows.
        watcher = ReaderCacheWatcher(session.cache.databaseUpdatesPublisher()) { [weak self] in
            await self?.reloadFromCache()
        }
        await reloadFromCache()
    }

    /// Fetches what's missing: every saved post's content, and its images when asked.
    func download(includeImages: Bool) async {
        await perform {
            if includeImages {
                try self.saved.setOfflineImageStore(
                    directory: ReaderOfflineImages.directory().path,
                    maxBytes: ReaderOfflineImages.maxBytes
                )
            }
            let result = try await self.saved.downloadForOffline(includeImages: includeImages)
            var parts = [
                "\(result.ready) ready",
                "\(result.withImages) with images",
                "\(result.failed) failed",
                "\(result.notDownloaded) not downloaded"
            ]
            if let reason = result.stoppedBecause {
                parts.append("stopped: \(reason)")
            }
            self.report = parts.joined(separator: ", ")
        }
    }

    func retry(_ row: ReaderOfflineRow) async {
        await perform {
            let state = try await self.saved.retryOffline(key: row.key)
            self.report = "\(row.title): \(state.label)"
        }
    }

    func removeImages() async {
        await perform {
            // The store has to be known for its files to be deleted.
            try self.saved.setOfflineImageStore(
                directory: ReaderOfflineImages.directory().path,
                maxBytes: ReaderOfflineImages.maxBytes
            )
            try self.saved.removeOfflineImages()
            self.report = "The stored images were deleted."
        }
    }

    /// Loads the next page of the list in the chosen order.
    func loadMore() async {
        await perform {
            _ = try await self.collection.loadNextPage()
        }
    }

    private func reloadFromCache() async {
        do {
            rows = try await collection.loadItems().map(ReaderOfflineRow.init)
            imageBytes = try saved.offlineImageBytes()
        } catch {
            errorMessage = readerErrorMessage(error)
        }
    }

    private func perform(_ work: () async throws -> Void) async {
        isWorking = true
        defer { isWorking = false }
        do {
            try await work()
        } catch {
            errorMessage = readerErrorMessage(error)
        }
        await reloadFromCache()
    }
}

struct ReaderOfflineView: View {

    @State
    var viewModel: ReaderOfflineViewModel

    var body: some View {
        List {
            Section("Download") {
                Button("Download text") {
                    Task { await viewModel.download(includeImages: false) }
                }
                .accessibilityIdentifier("offline-download-text")
                Button("Download text and images") {
                    Task { await viewModel.download(includeImages: true) }
                }
                .accessibilityIdentifier("offline-download-images")
                Button("Delete stored images", role: .destructive) {
                    Task { await viewModel.removeImages() }
                }
                .accessibilityIdentifier("offline-remove-images")
                LabeledContent("Images stored") {
                    Text(
                        "\(viewModel.imageBytes.formatted(.byteCount(style: .file))) of "
                            + ReaderOfflineImages.maxBytes.formatted(.byteCount(style: .file))
                    )
                }
                .accessibilityIdentifier("offline-image-bytes")
                if viewModel.isWorking {
                    HStack {
                        ProgressView()
                        Text("Working…")
                    }
                }
                if let report = viewModel.report {
                    ReaderNoticeRow(message: report)
                }
            }
            .disabled(viewModel.isWorking)

            Section("Order") {
                Picker("Order", selection: $viewModel.order) {
                    ForEach(ReaderOfflineOrder.allCases) { order in
                        Text(order.rawValue).tag(order)
                    }
                }
                .accessibilityIdentifier("offline-order")
                Button("Load more in this order") {
                    Task { await viewModel.loadMore() }
                }
                .disabled(viewModel.isWorking)
            }

            Section(viewModel.rows.count == 1 ? "1 saved post" : "\(viewModel.rows.count) saved posts") {
                ForEach(viewModel.rows) { row in
                    VStack(alignment: .leading, spacing: 4) {
                        if row.state.isReady {
                            NavigationLink(value: ReaderRoute.offlinePost(key: row.key)) {
                                Text(row.title)
                            }
                            .accessibilityIdentifier("offline-post")
                        } else {
                            Text(row.title)
                        }
                        if !row.detail.isEmpty {
                            Text(row.detail)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Text(row.state.label)
                            .font(.caption)
                            .foregroundStyle(row.state.isFailed ? .red : .secondary)
                            .accessibilityIdentifier("offline-state")
                        if row.state.isFailed {
                            Button("Try again") {
                                Task { await viewModel.retry(row) }
                            }
                            .buttonStyle(.borderless)
                            .disabled(viewModel.isWorking)
                        }
                    }
                }
            }
        }
        .navigationTitle("Offline Reading")
        .task { await viewModel.start() }
        .alert(
            "That didn't work",
            isPresented: Binding(
                get: { viewModel.errorMessage != nil },
                set: { if !$0 { viewModel.errorMessage = nil } }
            )
        ) {
            Button("OK") {}
        } message: {
            Text(viewModel.errorMessage ?? "")
        }
    }
}

/// A saved post as the device has it: its stored content, with its stored images. It makes no request.
struct ReaderOfflinePostView: View {
    let session: ReaderSession
    let key: ReaderSavedKey

    @State
    private var post: ReaderOfflinePost?

    @State
    private var loadError: String?

    var body: some View {
        Group {
            if let post {
                VStack(alignment: .leading, spacing: 0) {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(post.post.displayTitle)
                            .font(.headline)
                        Text("Stored images: \(post.images.count). Shown from this device, with no request.")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .accessibilityIdentifier("offline-post-images")
                    }
                    .padding()
                    if let content = post.post.content {
                        ReaderOfflineContentView(html: content)
                    } else {
                        ContentUnavailableView(
                            "No content",
                            systemImage: "doc",
                            description: Text("WordPress.com has no content for this save.")
                        )
                    }
                }
            } else if let loadError {
                ContentUnavailableView("The post couldn't be shown", systemImage: "xmark", description: Text(loadError))
            } else {
                ProgressView()
            }
        }
        .navigationTitle("Offline Copy")
        .task { load() }
    }

    private func load() {
        do {
            guard
                let found = try session.service.saved()
                    .offlinePost(key: key, imageUrlPrefix: ReaderOfflineImages.urlPrefix)
            else {
                loadError = "This post isn't saved, or the device doesn't have it yet."
                return
            }
            post = found
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

#if canImport(WebKit)

/// Serves a stored image to the web view, from the directory the images are kept in.
final class ReaderOfflineImageHandler: NSObject, WKURLSchemeHandler {
    func webView(_ webView: WKWebView, start urlSchemeTask: any WKURLSchemeTask) {
        do {
            guard let url = urlSchemeTask.request.url else {
                throw URLError(.badURL)
            }
            // Only a file's name is read from the address, so nothing outside the directory can be asked for.
            let file = try ReaderOfflineImages.directory().appendingPathComponent(url.lastPathComponent)
            let data = try Data(contentsOf: file)
            let response = URLResponse(
                url: url,
                mimeType: Self.mimeType(forExtension: file.pathExtension),
                expectedContentLength: data.count,
                textEncodingName: nil
            )
            urlSchemeTask.didReceive(response)
            urlSchemeTask.didReceive(data)
            urlSchemeTask.didFinish()
        } catch {
            urlSchemeTask.didFailWithError(error)
        }
    }

    func webView(_ webView: WKWebView, stop urlSchemeTask: any WKURLSchemeTask) {}

    private static func mimeType(forExtension fileExtension: String) -> String {
        switch fileExtension.lowercased() {
        case "jpg": "image/jpeg"
        case "png": "image/png"
        case "gif": "image/gif"
        case "webp": "image/webp"
        case "avif": "image/avif"
        case "svg": "image/svg+xml"
        case "bmp": "image/bmp"
        case "ico": "image/x-icon"
        case "heic": "image/heic"
        default: "application/octet-stream"
        }
    }
}

/// A saved post's stored content, as a web page. Scripts don't run, and its stored images come from the device.
struct ReaderOfflineContentView {
    let html: String

    final class Coordinator {
        var loadedHTML: String?
    }

    @MainActor
    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    @MainActor
    fileprivate func makeWebView() -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.defaultWebpagePreferences.allowsContentJavaScript = false
        configuration.setURLSchemeHandler(ReaderOfflineImageHandler(), forURLScheme: ReaderOfflineImages.scheme)
        return WKWebView(frame: .zero, configuration: configuration)
    }

    @MainActor
    fileprivate func update(_ webView: WKWebView, coordinator: Coordinator) {
        guard coordinator.loadedHTML != html else { return }
        coordinator.loadedHTML = html
        webView.loadHTMLString(Self.page(body: html), baseURL: nil)
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
extension ReaderOfflineContentView: NSViewRepresentable {
    func makeNSView(context: Context) -> WKWebView {
        makeWebView()
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator)
    }
}
#else
extension ReaderOfflineContentView: UIViewRepresentable {
    func makeUIView(context: Context) -> WKWebView {
        makeWebView()
    }

    func updateUIView(_ webView: WKWebView, context: Context) {
        update(webView, coordinator: context.coordinator)
    }
}
#endif

#else

/// A saved post's stored content, as plain text: this platform has no web view.
struct ReaderOfflineContentView: View {
    let html: String

    var body: some View {
        ScrollView {
            Text(html.readerPlainText)
                .padding()
        }
    }
}

#endif
