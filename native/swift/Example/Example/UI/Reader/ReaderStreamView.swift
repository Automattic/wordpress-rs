import SwiftUI
import WordPressAPI

/// Any Reader stream: its status, its posts and cards from the cache, and what a reader does to a post.
struct ReaderStreamView: View {

    @State
    var viewModel: ReaderStreamViewModel

    var body: some View {
        List {
            statusSection
            if let header = viewModel.header {
                headerSection(header)
            }
            Section(readerCount(viewModel.postCount, "post")) {
                ForEach(viewModel.entries) { entry in
                    row(for: entry)
                }
                if viewModel.hasMore {
                    Button("Load more") {
                        Task { await viewModel.loadMore() }
                    }
                    .disabled(viewModel.isWorking)
                    .accessibilityIdentifier("stream-load-more")
                }
            }
        }
        .navigationTitle(viewModel.stream.title)
        .refreshable { await viewModel.refresh() }
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

    private var statusSection: some View {
        Section("Status") {
            HStack {
                Text(viewModel.info?.state.label ?? "Not loaded")
                    .accessibilityIdentifier("stream-state")
                if viewModel.isWorking {
                    ProgressView()
                }
                Spacer()
                Button("Refresh") {
                    Task { await viewModel.refresh() }
                }
                .buttonStyle(.borderless)
                .disabled(viewModel.isWorking)
                .accessibilityIdentifier("stream-refresh")
            }
            LabeledContent(
                "Last refreshed",
                value: viewModel.info?.lastRefreshedAt?.readerLabel ?? "Never"
            )
            if let error = viewModel.loadError ?? viewModel.info?.errorMessage {
                Text("Last error: \(error)")
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("stream-error")
            }
            if let unreadable = viewModel.info?.unreadablePostCount, unreadable > 0 {
                HStack {
                    Text("\(readerCount(unreadable, "post")) couldn't be read")
                    Spacer()
                    Button("Try again") {
                        Task { await viewModel.retryUnreadablePosts() }
                    }
                    .buttonStyle(.borderless)
                    .disabled(viewModel.isWorking)
                }
            }
            if let notice = viewModel.notice {
                ReaderNoticeRow(message: notice)
            }
        }
    }

    private func headerSection(_ header: ReaderStreamHeader) -> some View {
        Section("About") {
            Text(header.title)
                .font(.headline)
            if let description = header.description, !description.isEmpty {
                Text(description)
            }
            if let subscribers = header.subscribers {
                LabeledContent("Subscribers", value: "\(subscribers)")
            }
            if let isFollowing = header.isFollowing {
                Button(isFollowing ? "Unfollow" : "Follow") {
                    Task { await viewModel.setHeaderFollowed(!isFollowing) }
                }
                .disabled(viewModel.isWorking)
                .accessibilityIdentifier("stream-header-follow")
            }
        }
    }

    @ViewBuilder
    private func row(for entry: ReaderStreamEntry) -> some View {
        switch entry {
        case .gap(let below):
            Button("Load missing posts") {
                Task { await viewModel.fillGap(below: below) }
            }
            .disabled(viewModel.isWorking)
        case .post(let post, let isSaved):
            ReaderStreamPostRow(post: post, isSaved: isSaved, viewModel: viewModel)
        case .cardTitle(_, let title):
            Text(title)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(.secondary)
        case .cardSite(_, let site):
            siteRow(site)
        case .cardTag(_, let tag):
            tagRow(tag)
        }
    }

    private func siteRow(_ site: ReaderStreamCardSite) -> some View {
        HStack {
            let title = site.name?.decodingHTMLEntities() ?? site.url ?? "A site"
            if let stream = site.stream {
                NavigationLink(value: ReaderRoute.stream(stream)) {
                    siteLabel(title: title, site: site)
                }
            } else {
                siteLabel(title: title, site: site)
            }
            let isFollowing = !viewModel.followedSites.isDisjoint(with: site.followKeys)
            Button(isFollowing ? "Unfollow" : "Follow") {
                Task { await viewModel.setFollowed(site.followTarget, !isFollowing) }
            }
            .buttonStyle(.borderless)
        }
    }

    private func siteLabel(title: String, site: ReaderStreamCardSite) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
            if let subscribers = site.subscribersCount {
                Text(readerCount(subscribers, "subscriber"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private func tagRow(_ tag: ReaderStreamCardTag) -> some View {
        HStack {
            let title = tag.title?.decodingHTMLEntities() ?? tag.slug
            NavigationLink(title, value: ReaderRoute.stream(.tag(slug: tag.slug)))
            let isFollowing = viewModel.followedTags.contains(tag.slug)
            Button(isFollowing ? "Unfollow" : "Follow") {
                Task { await viewModel.setTagFollowed(slug: tag.slug, title: tag.title, !isFollowing) }
            }
            .buttonStyle(.borderless)
        }
    }
}

/// A post in a stream, with a menu of what the reader can do to it.
private struct ReaderStreamPostRow: View {
    let post: ReaderPostRecord
    let isSaved: Bool
    let viewModel: ReaderStreamViewModel

    @Environment(ReaderStore.self)
    private var store

    var body: some View {
        HStack(alignment: .top) {
            NavigationLink(value: ReaderRoute.post(post)) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(post.displayTitle)
                        .font(.headline)
                    Text(byline)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if let summary = post.summary, !summary.isEmpty {
                        Text(summary)
                            .font(.subheadline)
                            .lineLimit(3)
                    }
                    Text(counts)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    marks
                }
            }
            .accessibilityIdentifier("stream-post")
            #if os(tvOS)
            // The remote can't move from the row to the menu beside it.
            .contextMenu { actionItems }
            #endif
            actions
        }
    }

    private var byline: String {
        [post.siteName?.decodingHTMLEntities(), post.authorName, post.date?.readerLabel]
            .compactMap { $0 }
            .joined(separator: " · ")
    }

    private var counts: String {
        [
            post.likeCount.map { readerCount($0, "like") },
            post.commentCount.map { readerCount($0, "comment") },
            post.readingTimeMinutes.map { "\($0) min read" },
            post.cardType.map { "\($0.label) card" }
        ]
        .compactMap { $0 }
        .joined(separator: " · ")
    }

    private var marks: some View {
        HStack(spacing: 4) {
            if let isSeen = post.isSeen {
                ReaderBadge(isSeen ? "Seen" : "Unseen")
            }
            if post.iLike == true {
                ReaderBadge("Liked")
            }
            if isSaved {
                ReaderBadge("Saved")
            }
            if post.isFollowing == true {
                ReaderBadge("Following")
            }
        }
    }

    private var actions: some View {
        Menu {
            actionItems
        } label: {
            Image(systemName: "ellipsis.circle")
                .accessibilityLabel("Actions for \(post.displayTitle)")
        }
        .buttonStyle(.borderless)
        .accessibilityIdentifier("stream-post-actions")
    }

    @ViewBuilder
    private var actionItems: some View {
        let liked = post.iLike == true
        Button(liked ? "Unlike" : "Like") {
            Task { await viewModel.setLiked(post, !liked, countsPageViews: store.countsPageViews) }
        }
        Button(isSaved ? "Unsave" : "Save") {
            Task { await viewModel.setSaved(post, !isSaved) }
        }
        // Only a post from a stream that sends a seen state has one to change.
        if let isSeen = post.isSeen {
            Button(isSeen ? "Mark as unseen" : "Mark as seen") {
                Task { await viewModel.setSeen(post, !isSeen) }
            }
        }
        let following = post.isFollowing == true
        Button(following ? "Unfollow site" : "Follow site") {
            Task { await viewModel.setFollowed(post, !following) }
        }
        if post.wordPressSiteID != nil {
            Button("Block site", role: .destructive) {
                Task { await viewModel.blockSite(of: post) }
            }
        }
        if let authorID = post.authorId, authorID != 0 {
            Button("Block author", role: .destructive) {
                Task { await viewModel.blockAuthor(of: post) }
            }
        }
    }
}
