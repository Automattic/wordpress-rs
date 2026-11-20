import SwiftUI
import WordPressAPI
import WordPressApiCache

/// The sites and feeds the account follows, from the cache, with how each site's news reaches the reader.
@MainActor
@Observable
final class ReaderSubscriptionsViewModel {
    let session: ReaderSession

    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false

    private(set) var subscriptions: [ReaderSubscriptionRecord] = []
    private(set) var notice: String?
    private var working = 0

    var filter = ""
    /// What went wrong fetching the list, shown in its status.
    var loadError: String?
    /// What went wrong with something the reader did, shown as an alert.
    var actionError: String?

    init(session: ReaderSession) {
        self.session = session
    }

    private var service: ReaderService { session.service }

    var isWorking: Bool { working > 0 }

    /// The subscriptions whose name or address has the filter's text.
    var shown: [ReaderSubscriptionRecord] {
        let text = filter.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !text.isEmpty else { return subscriptions }
        return subscriptions.filter { subscription in
            [subscription.name, subscription.url, subscription.feedUrl]
                .compactMap { $0?.lowercased() }
                .contains { $0.contains(text) }
        }
    }

    /// Shows what the cache has straight away, then fetches the list.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        // The list of subscriptions has no collection to ask whether a change is its own, so any change reloads.
        watcher = ReaderCacheWatcher(session.cache.databaseUpdatesPublisher()) { [weak self] in
            self?.reloadFromCache()
        }
        reloadFromCache()
        await refresh()
    }

    func refresh() async {
        working += 1
        defer { working -= 1 }
        do {
            let count = try await service.refreshSubscriptions()
            notice = "The account follows \(count) sites and feeds in all."
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
        reloadFromCache()
    }

    func unfollow(_ subscription: ReaderSubscriptionRecord) async {
        let target = ReaderFollowTarget(
            siteId: subscription.siteId,
            feedId: subscription.feedId,
            url: subscription.url ?? subscription.feedUrl,
            name: subscription.name
        )
        await act("Unfollow") {
            try await self.service.setFollowed(target: target, followed: false).message
        }
    }

    func setPostEmails(siteID: UInt64, _ enabled: Bool) async {
        await act("Emails for new posts") {
            try await self.service.setPostEmails(siteId: siteID, enabled: enabled).message
        }
    }

    func setCommentEmails(siteID: UInt64, _ enabled: Bool) async {
        await act("Emails for new comments") {
            try await self.service.setCommentEmails(siteId: siteID, enabled: enabled).message
        }
    }

    func setPostNotifications(siteID: UInt64, _ enabled: Bool) async {
        await act("Notifications for new posts") {
            try await self.service.setPostNotifications(siteId: siteID, enabled: enabled).message
        }
    }

    func setEmailFrequency(siteID: UInt64, _ frequency: ReaderSubscriptionEmailFrequency) async {
        await act("Email frequency") {
            // Unlike the switches, this one isn't kept to send later.
            try await self.service.setPostEmailFrequency(siteId: siteID, frequency: frequency)
            return "The server has it."
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
            subscriptions = try service.subscriptions()
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

struct ReaderSubscriptionsView: View {

    @State
    var viewModel: ReaderSubscriptionsViewModel

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
                        .accessibilityIdentifier("subscriptions-error")
                }
                if let notice = viewModel.notice {
                    ReaderNoticeRow(message: notice)
                }
                TextField("Filter by name or address", text: $viewModel.filter)
                    .autocorrectionDisabled()
                    .readerPlainInput()
                    .accessibilityIdentifier("subscriptions-filter")
            }

            Section("\(viewModel.shown.count) of \(viewModel.subscriptions.count) subscriptions") {
                ForEach(viewModel.shown, id: \.rowID) { subscription in
                    ReaderSubscriptionRowView(subscription: subscription, viewModel: viewModel)
                }
            }
        }
        .navigationTitle("Subscriptions")
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

private struct ReaderSubscriptionRowView: View {
    let subscription: ReaderSubscriptionRecord
    let viewModel: ReaderSubscriptionsViewModel

    @Environment(\.openReaderRoute)
    private var open

    @State
    private var showsDeliverySettings = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(subscription.name?.decodingHTMLEntities() ?? subscription.url ?? "A subscription")
                .font(.headline)
            if let address = subscription.url ?? subscription.feedUrl {
                Text(address)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if let unseen = subscription.unseenCount {
                Text("\(unseen) not seen")
                    .font(.caption)
                    .accessibilityIdentifier("subscription-unseen-count")
            }
            if subscription.subscriptionId == nil {
                ReaderBadge("Not sent yet")
            }
            HStack(spacing: 16) {
                if let stream = subscription.stream {
                    Button("Open") { open(.stream(stream)) }
                }
                Button("Unfollow", role: .destructive) {
                    Task { await viewModel.unfollow(subscription) }
                }
            }
            .buttonStyle(.borderless)

            // Only a WordPress.com or Jetpack site has delivery settings.
            if let siteID = subscription.siteId, siteID != 0 {
                Button(showsDeliverySettings ? "Hide delivery settings" : "Delivery settings") {
                    showsDeliverySettings.toggle()
                }
                .buttonStyle(.borderless)
                if showsDeliverySettings {
                    deliverySettings(siteID: siteID)
                }
            }
        }
    }

    @ViewBuilder
    private func deliverySettings(siteID: UInt64) -> some View {
        Toggle(
            "Email me new posts",
            isOn: Binding(
                get: { subscription.emailPosts == true },
                set: { enabled in Task { await viewModel.setPostEmails(siteID: siteID, enabled) } }
            )
        )
        Picker(
            "Email frequency",
            selection: Binding<ReaderSubscriptionEmailFrequency?>(
                get: { subscription.emailFrequency },
                set: { frequency in
                    // "Not set" is only shown, never sent: there's no frequency to set it to.
                    guard let frequency else { return }
                    Task { await viewModel.setEmailFrequency(siteID: siteID, frequency) }
                }
            )
        ) {
            // The server keeps no frequency while the site's post emails are off.
            if subscription.emailFrequency == nil {
                Text("Not set").tag(ReaderSubscriptionEmailFrequency?.none)
            }
            ForEach(frequencies, id: \.self) { frequency in
                Text(frequency.label).tag(ReaderSubscriptionEmailFrequency?.some(frequency))
            }
        }
        .disabled(subscription.emailPosts != true)
        Toggle(
            "Email me new comments",
            isOn: Binding(
                get: { subscription.emailComments == true },
                set: { enabled in Task { await viewModel.setCommentEmails(siteID: siteID, enabled) } }
            )
        )
        Toggle(
            "Notify me of new posts",
            isOn: Binding(
                get: { subscription.notifyPosts == true },
                set: { enabled in Task { await viewModel.setPostNotifications(siteID: siteID, enabled) } }
            )
        )
    }

    /// The frequencies to pick from, with the subscription's own when it's one this app doesn't know.
    private var frequencies: [ReaderSubscriptionEmailFrequency] {
        let known: [ReaderSubscriptionEmailFrequency] = [.instantly, .daily, .weekly]
        guard let current = subscription.emailFrequency, !known.contains(current) else { return known }
        return known + [current]
    }
}

extension ReaderSubscriptionRecord {
    /// A stable name for the subscription, to tell rows apart.
    fileprivate var rowID: String {
        if let siteId, siteId != 0 {
            return "site:\(siteId)"
        }
        if let feedId {
            return "feed:\(feedId)"
        }
        return "address:\(url ?? feedUrl ?? "")"
    }

    /// The site's stream, or the feed's when there's no site.
    fileprivate var stream: ReaderStream? {
        if let siteId, siteId != 0 {
            return .site(siteId: siteId)
        }
        return feedId.map { .feed(feedId: $0) }
    }
}
