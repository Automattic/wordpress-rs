import SwiftUI
import WordPressAPI

/// A site or feed the search found, with whether the reader follows it.
struct ReaderFoundSite: Identifiable {
    let id: Int
    let result: ReaderFeedSearchResult
    var isFollowing: Bool

    var title: String {
        (result.title ?? result.site?.name ?? result.feed?.name)?.decodingHTMLEntities() ?? address ?? "A site"
    }

    /// Some results leave out their own address, but their site or feed has it.
    var address: String? {
        result.url ?? result.site?.url ?? result.feed?.url ?? result.subscribeUrl
    }

    var subscribersCount: UInt64? {
        result.subscribersCount ?? result.feed?.subscribersCount
    }

    /// The site's stream, or its feed's when it has no site ID.
    var stream: ReaderStream? {
        if let siteId = result.siteId, siteId != 0 {
            return .site(siteId: siteId)
        }
        return result.feedId.map { .feed(feedId: $0) }
    }

    var followTarget: ReaderFollowTarget {
        ReaderFollowTarget(
            siteId: result.siteId,
            feedId: result.feedId,
            url: result.subscribeUrl ?? address,
            name: title
        )
    }
}

/// The search that finds sites and feeds to follow. Its results aren't cached.
@MainActor
@Observable
final class ReaderFindSitesViewModel {
    let session: ReaderSession

    private(set) var sites: [ReaderFoundSite] = []
    private(set) var total: UInt64?
    private(set) var nextOffset: UInt32?
    private(set) var searchedFor: String?
    private(set) var notice: String?
    private var working = 0

    var query = ""
    /// What went wrong searching, shown in the status.
    var loadError: String?
    /// What went wrong following a site, shown as an alert.
    var actionError: String?

    init(session: ReaderSession) {
        self.session = session
    }

    var isWorking: Bool { working > 0 }

    func search() async {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return }
        sites = []
        total = nil
        nextOffset = nil
        searchedFor = query
        await loadPage(query: query, offset: nil)
    }

    func loadMore() async {
        guard let searchedFor, let nextOffset else { return }
        await loadPage(query: searchedFor, offset: nextOffset)
    }

    func setFollowed(_ site: ReaderFoundSite, _ followed: Bool) async {
        working += 1
        defer { working -= 1 }
        do {
            let outcome = try await session.service.setFollowed(target: site.followTarget, followed: followed)
            notice = "\(followed ? "Follow" : "Unfollow"): \(outcome.message)"
            // The results aren't cached, so what the reader did is kept here.
            if let index = sites.firstIndex(where: { $0.id == site.id }) {
                sites[index].isFollowing = followed
            }
        } catch {
            actionError = "\(followed ? "Follow" : "Unfollow") failed. \(readerErrorMessage(error))"
        }
    }

    private func loadPage(query: String, offset: UInt32?) async {
        working += 1
        defer { working -= 1 }
        do {
            let page = try await session.service.searchSites(query: query, offset: offset)
            // A search started since has replaced this one.
            guard searchedFor == query else { return }
            sites += page.feeds.enumerated()
                .map { index, result in
                    ReaderFoundSite(
                        id: sites.count + index,
                        result: result,
                        isFollowing: result.site?.isFollowing ?? result.feed?.isFollowing ?? false
                    )
                }
            total = page.total ?? total
            nextOffset = page.nextOffset
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

struct ReaderFindSitesView: View {

    @State
    var viewModel: ReaderFindSitesViewModel

    var body: some View {
        List {
            Section("Search") {
                TextField("A name, a topic or an address", text: $viewModel.query)
                    .autocorrectionDisabled()
                    .readerPlainInput()
                    .accessibilityIdentifier("find-sites-query")
                    .onSubmit { Task { await viewModel.search() } }
                Button("Find sites") {
                    Task { await viewModel.search() }
                }
                .disabled(viewModel.query.isEmpty || viewModel.isWorking)
                .accessibilityIdentifier("find-sites")
                if viewModel.isWorking {
                    ProgressView()
                }
                if let error = viewModel.loadError {
                    Text("Last error: \(error)")
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("find-sites-error")
                }
                if let notice = viewModel.notice {
                    ReaderNoticeRow(message: notice)
                }
            }

            if let searchedFor = viewModel.searchedFor {
                Section(title(for: searchedFor)) {
                    ForEach(viewModel.sites) { site in
                        row(for: site)
                    }
                    if viewModel.nextOffset != nil {
                        Button("Load more") {
                            Task { await viewModel.loadMore() }
                        }
                        .disabled(viewModel.isWorking)
                        .accessibilityIdentifier("find-sites-load-more")
                    }
                }
            }
        }
        .navigationTitle("Find sites")
        .scrollDismissesKeyboard(.immediately)
        .alert(
            "That didn't work",
            isPresented: Binding(
                get: { viewModel.actionError != nil },
                set: { if !$0 { viewModel.actionError = nil } }
            )
        ) {
            Button("OK") {}
        } message: {
            Text(viewModel.actionError ?? "")
        }
    }

    private func title(for query: String) -> String {
        let total = viewModel.total.map { " of \($0)" } ?? ""
        return "\(viewModel.sites.count)\(total) found for \(query)"
    }

    private func row(for site: ReaderFoundSite) -> some View {
        HStack {
            if let stream = site.stream {
                NavigationLink(value: ReaderRoute.stream(stream)) {
                    label(for: site)
                }
            } else {
                label(for: site)
            }
            Button(site.isFollowing ? "Unfollow" : "Follow") {
                Task { await viewModel.setFollowed(site, !site.isFollowing) }
            }
            .buttonStyle(.borderless)
            .disabled(viewModel.isWorking)
        }
    }

    private func label(for site: ReaderFoundSite) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(site.title)
            Text(
                [site.address, site.subscribersCount.map { readerCount($0, "subscriber") }]
                    .compactMap { $0 }
                    .joined(separator: " · ")
            )
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }
}
