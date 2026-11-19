import SwiftUI
import WordPressAPI

/// The Reader home: who the reader is, what's waiting to be sent, and every way into the Reader.
struct ReaderHomeView: View {

    @State
    var viewModel: ReaderHomeViewModel

    @Binding
    var path: [ReaderRoute]

    /// Whether to read signed out although the app has an account, when it has one.
    let readerPicker: Binding<Bool>?

    @Environment(ReaderStore.self)
    private var store

    var body: some View {
        List {
            readerSection
            streamsSection
            tagsSection
            suggestedTagsSection
            searchSection
            siteAndFeedSection
            postSection
            moreSection
            developerSection
        }
        .scrollDismissesKeyboard(.immediately)
        .refreshable { await viewModel.refreshMenu() }
        .task { await viewModel.start() }
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

    private var readerSection: some View {
        Section("Reader") {
            LabeledContent("Reading as", value: viewModel.session.name)
            if let readerPicker {
                Toggle("Read signed out", isOn: readerPicker)
            }
            LabeledContent("Waiting to be sent", value: "\(viewModel.pendingActions)")
                .accessibilityIdentifier("reader-pending-actions")
            Button("Sync") {
                Task { await viewModel.sync() }
            }
            .disabled(viewModel.isWorking)
            if !viewModel.session.isSignedOut {
                Button("Take over what was followed and saved signed out") {
                    Task { await viewModel.adoptSignedOut() }
                }
                .disabled(viewModel.isWorking)
            }
            if let notice = viewModel.notice {
                ReaderNoticeRow(message: notice)
            }
            if let error = viewModel.loadError {
                Text("Last error: \(error)")
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("reader-error")
            }
            if let error = store.signOutError {
                Text("Deleting the signed-out account's cached data failed: \(error)")
                    .foregroundStyle(.red)
            }
        }
    }

    private var streamsSection: some View {
        Group {
            Section("Streams") {
                if !viewModel.menuHasLoaded {
                    Text(viewModel.isWorking ? "Loading the menu…" : "The menu hasn't loaded yet.")
                        .foregroundStyle(.secondary)
                }
                ForEach(viewModel.streams) { entry in
                    menuRow(entry)
                }
                // These two aren't in the menu.
                NavigationLink("Freshly Pressed", value: ReaderRoute.stream(.freshlyPressed))
                NavigationLink("First Posts", value: ReaderRoute.stream(.firstPosts))
            }

            if !viewModel.lists.isEmpty {
                Section("Lists") {
                    ForEach(viewModel.lists) { entry in
                        menuRow(entry)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func menuRow(_ entry: ReaderMenuEntry) -> some View {
        if let stream = entry.stream {
            NavigationLink(entry.title, value: ReaderRoute.stream(stream))
        } else {
            LabeledContent(entry.title, value: "Can't be opened")
        }
    }

    private var tagsSection: some View {
        Section("Followed tags") {
            if let stream = viewModel.followedTagsStream {
                NavigationLink("Latest from my tags", value: ReaderRoute.stream(stream))
            } else {
                Text("No tags followed yet.")
                    .foregroundStyle(.secondary)
            }
            ForEach(viewModel.followedTags) { tag in
                HStack {
                    NavigationLink(tag.title, value: ReaderRoute.stream(.tag(slug: tag.slug)))
                    Button("Unfollow", role: .destructive) {
                        Task { await viewModel.setTagFollowed(slug: tag.slug, title: tag.title, followed: false) }
                    }
                    .buttonStyle(.borderless)
                    .accessibilityIdentifier("unfollow-tag-\(tag.slug)")
                }
            }
            HStack {
                TextField("Tag slug, such as knitting", text: $viewModel.tagToFollow)
                    .autocorrectionDisabled()
                    .readerPlainInput()
                    .accessibilityIdentifier("tag-slug")
                Button("Follow") {
                    Task { await viewModel.setTagFollowed(slug: viewModel.tagToFollow, title: nil, followed: true) }
                }
                .buttonStyle(.borderless)
                .disabled(viewModel.tagToFollow.isEmpty || viewModel.isWorking)
                .accessibilityIdentifier("follow-tag-by-slug")
            }
        }
    }

    private var suggestedTagsSection: some View {
        Group {
            if !viewModel.recommendedTags.isEmpty {
                Section("Recommended tags") {
                    ForEach(viewModel.recommendedTags) { tag in
                        suggestedTagRow(tag)
                    }
                }
            }

            Section("Interests") {
                if viewModel.interests.isEmpty {
                    Button("Load the tags a new reader picks from") {
                        Task { await viewModel.loadInterests() }
                    }
                    .disabled(viewModel.isWorking)
                }
                ForEach(viewModel.interests) { tag in
                    suggestedTagRow(tag)
                }
            }
        }
    }

    private func suggestedTagRow(_ tag: ReaderTagEntry) -> some View {
        HStack {
            NavigationLink(tag.title, value: ReaderRoute.stream(.tag(slug: tag.slug)))
            if tag.isFollowed {
                ReaderBadge("Following")
            } else {
                Button("Follow") {
                    Task { await viewModel.setTagFollowed(slug: tag.slug, title: tag.title, followed: true) }
                }
                .buttonStyle(.borderless)
                .accessibilityIdentifier("follow-tag-\(tag.slug)")
            }
        }
    }

    private var searchSection: some View {
        Section("Search posts") {
            TextField("Search for", text: $viewModel.searchQuery)
                .autocorrectionDisabled()
                .readerPlainInput()
                .accessibilityIdentifier("search-query")
            Picker("Order", selection: $viewModel.searchSort) {
                Text("Best match").tag(ReaderSearchSort.relevance)
                Text("Newest").tag(ReaderSearchSort.date)
            }
            Button("Search") {
                path.append(.stream(.search(query: searchQuery, sort: viewModel.searchSort)))
            }
            .disabled(searchQuery.isEmpty)
            .accessibilityIdentifier("search-posts")
        }
    }

    private var searchQuery: String {
        viewModel.searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var siteAndFeedSection: some View {
        Section("Open a site or a feed") {
            HStack {
                TextField("Site ID", text: $viewModel.siteID)
                    .readerNumberInput()
                    .accessibilityIdentifier("site-id")
                Button("Open site") {
                    if let siteID = UInt64(viewModel.siteID) {
                        path.append(.stream(.site(siteId: siteID)))
                    }
                }
                .buttonStyle(.borderless)
                .disabled(UInt64(viewModel.siteID) == nil)
            }
            HStack {
                TextField("Feed ID", text: $viewModel.feedID)
                    .readerNumberInput()
                    .accessibilityIdentifier("feed-id")
                Button("Open feed") {
                    if let feedID = UInt64(viewModel.feedID) {
                        path.append(.stream(.feed(feedId: feedID)))
                    }
                }
                .buttonStyle(.borderless)
                .disabled(UInt64(viewModel.feedID) == nil)
            }
        }
    }

    private var postSection: some View {
        Section("Open a post") {
            HStack {
                TextField("Post address (needs an account)", text: $viewModel.postURL)
                    .autocorrectionDisabled()
                    .readerPlainInput()
                    .accessibilityIdentifier("post-url")
                Button("Open") {
                    Task {
                        if let route = await viewModel.openPostAtURL() {
                            path.append(route)
                        }
                    }
                }
                .buttonStyle(.borderless)
                .disabled(viewModel.postURL.isEmpty || viewModel.isWorking)
                .accessibilityIdentifier("open-post-at-url")
            }
            TextField("Site ID or domain", text: $viewModel.postSite)
                .autocorrectionDisabled()
                .readerPlainInput()
                .accessibilityIdentifier("post-site")
            HStack {
                TextField("Post slug", text: $viewModel.postSlug)
                    .autocorrectionDisabled()
                    .readerPlainInput()
                    .accessibilityIdentifier("post-slug")
                Button("Open") {
                    Task {
                        if let route = await viewModel.openPostBySlug() {
                            path.append(route)
                        }
                    }
                }
                .buttonStyle(.borderless)
                .disabled(viewModel.postSite.isEmpty || viewModel.postSlug.isEmpty || viewModel.isWorking)
                .accessibilityIdentifier("open-post-by-slug")
            }
        }
    }

    private var moreSection: some View {
        Section("More") {
            NavigationLink("Saved posts", value: ReaderRoute.saved)
            NavigationLink("Subscriptions", value: ReaderRoute.subscriptions)
            NavigationLink("Blocked sites", value: ReaderRoute.blockedSites)
            NavigationLink("Find sites", value: ReaderRoute.findSites)
        }
    }

    private var developerSection: some View {
        @Bindable var store = store
        return Section {
            Toggle("Count page views", isOn: $store.countsPageViews)
            Toggle(
                "Simulate no connection",
                isOn: Binding(get: { store.isOffline }, set: { store.setOffline($0) })
            )
        } header: {
            Text("Developer")
        } footer: {
            Text(
                "With \"Count page views\" on, opening a post reports it, which may count a view on the post's "
                    + "site. A counted view can't be taken back."
            )
        }
    }
}
