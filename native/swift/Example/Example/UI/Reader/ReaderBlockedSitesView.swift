import SwiftUI
import WordPressAPI
import WordPressApiCache

/// The sites the account blocked, from the cache, and the users blocked on this device.
@MainActor
@Observable
final class ReaderBlockedSitesViewModel {
    let session: ReaderSession

    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false

    private(set) var sites: [ReaderBlockedSite] = []
    private(set) var users: [UInt64] = []
    private(set) var notice: String?
    private var working = 0

    var siteToBlock = ""
    /// What went wrong fetching the list, shown in its status.
    var loadError: String?
    /// What went wrong with something the reader did, shown as an alert.
    var actionError: String?

    init(session: ReaderSession) {
        self.session = session
    }

    private var service: ReaderService { session.service }

    var isWorking: Bool { working > 0 }

    /// Shows what the cache has straight away, then fetches the list.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        // The blocked sites have no collection to ask whether a change is theirs, so any change reloads.
        watcher = ReaderCacheWatcher(session.cache.databaseUpdatesPublisher()) { [weak self] in
            self?.reloadFromCache()
        }
        reloadFromCache()
        await refresh()
    }

    func refresh() async {
        // Blocked sites belong to an account. Signed out the server can only refuse, which isn't news to show as
        // an error: the screen says so instead, and still lists the users blocked on this device.
        guard !session.isSignedOut else {
            reloadFromCache()
            return
        }
        working += 1
        defer { working -= 1 }
        do {
            try await service.refreshBlockedSites()
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
        reloadFromCache()
    }

    func blockSiteByID() async {
        guard let siteID = UInt64(siteToBlock.trimmingCharacters(in: .whitespaces)) else { return }
        await act("Block") {
            let outcome = try await self.service.setSiteBlocked(siteId: siteID, blocked: true)
            self.siteToBlock = ""
            return outcome.message
        }
    }

    func unblock(_ site: ReaderBlockedSite) async {
        await act("Unblock") {
            try await self.service.setSiteBlocked(siteId: site.id, blocked: false).message
        }
    }

    func unblockUser(_ userID: UInt64) async {
        await act("Unblock user") {
            try self.service.setUserBlocked(userId: userID, blocked: false)
            return "Unblocked on this device."
        }
    }

    private func act(_ name: String, _ work: () async throws -> String) async {
        working += 1
        defer { working -= 1 }
        do {
            notice = "\(name): \(try await work())"
        } catch {
            actionError = "\(name) failed. \(readerErrorMessage(error))"
        }
        reloadFromCache()
    }

    private func reloadFromCache() {
        do {
            sites = try service.blockedSites()
            users = try service.blockedUsers()
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

struct ReaderBlockedSitesView: View {

    @State
    var viewModel: ReaderBlockedSitesViewModel

    var body: some View {
        List {
            Section("Status") {
                HStack {
                    Text(viewModel.isWorking ? "Working…" : "Idle")
                    if viewModel.isWorking {
                        ProgressView()
                    }
                }
                if let error = viewModel.loadError {
                    Text("Last error: \(error)")
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("blocked-sites-error")
                }
                if let notice = viewModel.notice {
                    ReaderNoticeRow(message: notice)
                }
            }

            Section(readerCount(viewModel.sites.count, "blocked site")) {
                if viewModel.session.isSignedOut {
                    Text("Sites are blocked by a WordPress.com account. Sign in to see or change them.")
                        .foregroundStyle(.secondary)
                        .accessibilityIdentifier("blocked-sites-signed-out")
                }
                ForEach(viewModel.sites, id: \.id) { site in
                    ReaderRowWithControls {
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(site.name?.decodingHTMLEntities() ?? "Site \(site.id)")
                                Text(site.url ?? "Site \(site.id)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                        }
                    } controls: {
                        Button("Unblock") {
                            Task { await viewModel.unblock(site) }
                        }
                        .disabled(viewModel.isWorking)
                    }
                }
                HStack {
                    TextField("Site ID to block", text: $viewModel.siteToBlock)
                        .readerNumberInput()
                        .accessibilityIdentifier("site-to-block")
                    Button("Block") {
                        Task { await viewModel.blockSiteByID() }
                    }
                    .buttonStyle(.borderless)
                    .disabled(UInt64(viewModel.siteToBlock) == nil || viewModel.isWorking)
                    .accessibilityIdentifier("block-site-by-id")
                }
            }

            Section {
                ForEach(viewModel.users, id: \.self) { userID in
                    ReaderRowWithControls {
                        HStack {
                            // Not a number to read: no thousands separators.
                            Text(verbatim: "User \(userID)")
                            Spacer()
                        }
                    } controls: {
                        Button("Unblock") {
                            Task { await viewModel.unblockUser(userID) }
                        }
                    }
                }
            } header: {
                Text(readerCount(viewModel.users.count, "blocked user"))
            } footer: {
                Text("Users are blocked on this device only: WordPress.com keeps no list of them.")
            }
        }
        .navigationTitle("Blocked sites")
        .refreshable { await viewModel.refresh() }
        .task { await viewModel.start() }
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
}
