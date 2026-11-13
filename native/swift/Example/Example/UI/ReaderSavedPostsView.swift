import SwiftUI
import WordPressAPI
import WordPressApiCache

struct ReaderSavedPostRow: Identifiable {
    let id: ReaderSavedKey
    let title: String
    let subtitle: String
    /// The post behind this row, which a save made just now doesn't have yet.
    let post: ReaderPostSaveIdentity?

    init(_ item: ReaderSavedListItem) {
        id = item.key
        post = item.post?.identity

        if let title = item.post?.title {
            self.title = title
        } else if let url = item.post?.identity.url {
            self.title = url
        } else if case .url(let url) = item.key {
            self.title = url
        } else {
            self.title = String(describing: item.key)
        }

        let saved = item.dateSaved.map { "Saved \($0.formatted())" }
        let pending =
            item.isWaitingForRoom
            ? "Waiting for room under the save limit" : item.isPending ? "Not sent yet" : nil
        subtitle = [item.post?.siteName, saved, pending]
            .compactMap { $0 }
            .joined(separator: " · ")
    }
}

/// The saved list from the cache, through the service's collection. The rows reload whenever the
/// cache changes in a way the list shows, so a save, an unsave, a refresh and a relaunch all show up
/// without any bookkeeping here.
@MainActor
@Observable
final class ReaderSavedPostsViewModel {
    typealias ServiceProvider = @Sendable () async throws -> (service: ReaderSavedService, cache: WordPressApiCache)

    private let serviceProvider: ServiceProvider

    private var service: ReaderSavedService?
    private var collection: ReaderSavedPostsCollection?
    private var updatesObserver: NSObjectProtocol?

    private(set) var rows: [ReaderSavedPostRow] = []
    private(set) var total: UInt64?
    private(set) var hasMore = false
    private(set) var isLoading = false

    var urlToSave = ""
    var error: MyError?
    var shouldPresentAlert = false

    init(serviceProvider: @escaping ServiceProvider) {
        self.serviceProvider = serviceProvider
    }

    /// Shows what the cache has straight away, then catches up with the server.
    func start() async {
        await perform {
            _ = try await self.loadedCollection()
            try await self.reloadFromCache()
            _ = try await self.loadedCollection().refresh()
        }
    }

    func reload() async {
        await perform {
            _ = try await self.loadedCollection().refresh()
        }
    }

    func loadMore() async {
        await perform {
            _ = try await self.loadedCollection().loadNextPage()
        }
    }

    func saveURL() async {
        let url = urlToSave.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return }

        await perform {
            _ = try await self.loadedService().saveUrl(url: url, pageDetails: nil)
            self.urlToSave = ""
        }
    }

    func unsave(_ row: ReaderSavedPostRow) async {
        await perform {
            let service = try await self.loadedService()
            // The post's identity covers every key it may be saved under.
            if let post = row.post {
                _ = try await service.unsave(post: post)
            } else {
                _ = try await service.unsaveKey(key: row.id)
            }
        }
    }

    private func reloadFromCache() async throws {
        guard let collection, let service else { return }
        rows = try await collection.loadItems().map(ReaderSavedPostRow.init)
        hasMore = try collection.listInfo().hasMore ?? false
        total = try service.savedCount()
    }

    private func loadedService() async throws -> ReaderSavedService {
        _ = try await loadedCollection()
        return service!
    }

    private func loadedCollection() async throws -> ReaderSavedPostsCollection {
        if let collection {
            return collection
        }

        let (service, cache) = try await serviceProvider()
        let collection = service.savedPosts()
        self.service = service
        self.collection = collection
        updatesObserver = cache.addDatabaseUpdatesObserver { [weak self] hook in
            guard collection.isRelevantUpdate(hook: hook) else { return }
            Task { @MainActor in try? await self?.reloadFromCache() }
        }
        return collection
    }

    private func perform(_ work: () async throws -> Void) async {
        isLoading = true
        defer { isLoading = false }

        do {
            try await work()
        } catch {
            self.error = MyError(underlyingError: error)
            self.shouldPresentAlert = true
        }
    }
}

struct ReaderSavedPostsView: View {

    @State
    var viewModel: ReaderSavedPostsViewModel

    var body: some View {
        List {
            Section {
                TextField("https://example.com/page", text: $viewModel.urlToSave)
                    .autocorrectionDisabled()
                    .onSubmit { Task { await viewModel.saveURL() } }
                Button("Save URL") {
                    Task { await viewModel.saveURL() }
                }
                .disabled(viewModel.urlToSave.isEmpty || viewModel.isLoading)
            }

            Section(viewModel.total.map { "\($0) saved" } ?? "Saved") {
                ForEach(viewModel.rows) { row in
                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(row.title)
                            Text(row.subtitle)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Button("Unsave", role: .destructive) {
                            Task { await viewModel.unsave(row) }
                        }
                        .buttonStyle(.borderless)
                    }
                }

                if viewModel.hasMore {
                    Button("Load More") {
                        Task { await viewModel.loadMore() }
                    }
                    .disabled(viewModel.isLoading)
                }
            }
        }
        .overlay {
            if viewModel.isLoading && viewModel.rows.isEmpty {
                ProgressView()
            }
        }
        .navigationTitle("Saved Posts")
        .refreshable { await viewModel.reload() }
        .task { await viewModel.start() }
        .alert(isPresented: $viewModel.shouldPresentAlert, error: viewModel.error) {
            Button("OK") {}
        }
    }
}
