import SwiftUI
import WordPressAPI

struct ReaderSavedPostRow: Identifiable {
    let id: ReaderSavedKey
    let title: String
    let subtitle: String
    /// The post behind this row, which a save made just now doesn't have yet.
    let post: ReaderPostSaveIdentity?
}

@MainActor
@Observable
final class ReaderSavedPostsViewModel {
    typealias ServiceProvider = @Sendable () async throws -> ReaderSavedService

    private static let pageSize: UInt32 = 20

    /// Passes the service's changes on to the main actor.
    private final class ChangeObserver: ReaderSavedObserver {
        private let onChange: @Sendable () -> Void

        init(onChange: @escaping @Sendable () -> Void) {
            self.onChange = onChange
        }

        func savedPostsChanged() {
            onChange()
        }
    }

    private let serviceProvider: ServiceProvider

    private var service: ReaderSavedService?
    private var listedPosts: [ReaderSavedPost] = []
    private var savedURLs: [ReaderSavedKey: String] = [:]
    /// Bumped when the service's state changes, because that state lives outside this type.
    private var revision = 0

    private(set) var isLoading = false
    private(set) var nextPageHandle: String?

    var urlToSave = ""
    var error: MyError?
    var shouldPresentAlert = false

    init(serviceProvider: @escaping ServiceProvider) {
        self.serviceProvider = serviceProvider
    }

    var total: UInt64? {
        _ = revision
        return service?.savedCount()
    }

    /// The service's recent saves above the list, without anything just unsaved. A save, an unsave
    /// or a failure of either shows up here without any bookkeeping in this type.
    var rows: [ReaderSavedPostRow] {
        _ = revision
        guard let service else { return [] }

        let listedRows =
            listedPosts
            .filter { service.status(post: $0.identity) != .notSaved }
            .compactMap(Self.row(for:))
        let listedKeys = Set(listedRows.map(\.id))
        let recentRows = service.recentSaves()
            .filter { !listedKeys.contains($0) }
            .map { key in
                ReaderSavedPostRow(
                    id: key,
                    title: savedURLs[key] ?? String(describing: key),
                    subtitle: "Saved just now",
                    post: nil
                )
            }

        return recentRows + listedRows
    }

    func reload() async {
        await perform {
            let service = try await self.loadedService()
            try await service.refresh()
            let page = try await service.listPosts(params: .init(number: Self.pageSize))

            self.listedPosts = page.posts
            self.nextPageHandle = page.nextPageHandle
        }
    }

    func loadMore() async {
        guard let nextPageHandle else { return }

        await perform {
            let service = try await self.loadedService()
            let page = try await service.listPosts(params: .init(pageHandle: nextPageHandle, number: Self.pageSize))

            self.listedPosts.append(contentsOf: page.posts)
            self.nextPageHandle = page.nextPageHandle
        }
    }

    func saveURL() async {
        let url = urlToSave.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return }

        await perform {
            let service = try await self.loadedService()
            let key = try await service.saveUrl(url: url, pageDetails: nil)

            self.savedURLs[key] = url
            self.urlToSave = ""
        }
    }

    func unsave(_ row: ReaderSavedPostRow) async {
        await perform {
            let service = try await self.loadedService()
            if let post = row.post {
                try await service.unsave(post: post)
            } else {
                try await service.unsaveKey(key: row.id)
            }
        }
    }

    private func loadedService() async throws -> ReaderSavedService {
        if let service {
            return service
        }

        let service = try await serviceProvider()
        service.setObserver(
            observer: ChangeObserver { [weak self] in
                Task { @MainActor in self?.revision += 1 }
            }
        )
        self.service = service
        return service
    }

    private static func row(for post: ReaderSavedPost) -> ReaderSavedPostRow? {
        guard let key = post.identity.savedKey else { return nil }

        return ReaderSavedPostRow(
            id: key,
            title: post.title ?? post.identity.url ?? "Untitled",
            subtitle: [post.siteName, post.dateSaved.map { "Saved \($0.formatted())" }]
                .compactMap { $0 }
                .joined(separator: " · "),
            post: post.identity
        )
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

                // Only a missing handle means the end: a page can be short, or even empty, with more to come.
                if viewModel.nextPageHandle != nil {
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
        .task { await viewModel.reload() }
        .alert(isPresented: $viewModel.shouldPresentAlert, error: viewModel.error) {
            Button("OK") {}
        }
    }
}
