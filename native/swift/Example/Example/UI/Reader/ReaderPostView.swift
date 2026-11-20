import SwiftUI
import WordPressAPI

/// A post: what the cache has of it, what the reader can do to it, and the ways on from it.
struct ReaderPostView: View {

    @State
    var viewModel: ReaderPostViewModel

    @Environment(ReaderStore.self)
    private var store

    var body: some View {
        List {
            statusSection
            if let post = viewModel.post {
                detailsSection(post)
                actionsSection(post)
                moreSection(post)
                tagsSection(post)
                contentSection(post)
            }
        }
        .navigationTitle("Post")
        .refreshable { await viewModel.refresh() }
        .task { await viewModel.start(countsPageViews: store.countsPageViews) }
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
                Text(status)
                    .accessibilityIdentifier("post-state")
                if viewModel.isWorking {
                    ProgressView()
                }
            }
            if let error = viewModel.loadError {
                Text("Last error: \(error)")
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("post-error")
            }
            if let seen = viewModel.seenOnOpen {
                LabeledContent("Seen on opening", value: seen)
                    .accessibilityIdentifier("post-seen-on-open")
            }
            if let pageView = viewModel.pageView {
                LabeledContent("Page view", value: pageView)
            }
            if let notice = viewModel.notice {
                ReaderNoticeRow(message: notice)
            }
        }
    }

    private var status: String {
        if viewModel.isWorking {
            return viewModel.post == nil ? "Fetching the post…" : "From the cache. Fetching it again…"
        }
        if viewModel.post == nil {
            return "The cache doesn't have this post."
        }
        return viewModel.hasFetched ? "Up to date with the server." : "From the cache."
    }

    private func detailsSection(_ post: ReaderPostRecord) -> some View {
        Section("Post") {
            Text(post.displayTitle)
                .font(.headline)
            if let author = post.authorName {
                LabeledContent("Author", value: author)
            }
            if let site = post.siteName?.decodingHTMLEntities() {
                LabeledContent("Site", value: site)
            }
            if let date = post.date {
                LabeledContent("Date", value: date.readerLabel)
            }
            LabeledContent("Likes", value: post.likeCount.map { "\($0)" } ?? "Unknown")
            LabeledContent("Comments", value: post.commentCount.map { "\($0)" } ?? "Unknown")
            if let words = post.wordCount {
                LabeledContent("Words", value: "\(words)")
            }
            if let minutes = post.readingTimeMinutes {
                LabeledContent("Reading time", value: "\(minutes) min")
            }
            if let cardType = post.cardType {
                LabeledContent("Card", value: cardType.label)
            }
        }
    }

    private func actionsSection(_ post: ReaderPostRecord) -> some View {
        Section("Actions") {
            let liked = post.iLike == true
            toggleRow("Liked", isOn: liked, turnOn: "Like", turnOff: "Unlike") {
                await viewModel.setLiked(!liked, countsPageViews: store.countsPageViews)
            }
            toggleRow("Saved", isOn: viewModel.isSaved, turnOn: "Save", turnOff: "Unsave") {
                await viewModel.setSaved(!viewModel.isSaved)
            }
            if let isSeen = post.isSeen {
                toggleRow("Seen", isOn: isSeen, turnOn: "Mark as seen", turnOff: "Mark as unseen") {
                    await viewModel.setSeen(!isSeen)
                }
            } else {
                LabeledContent("Seen", value: "This post has no seen state")
            }
            let following = post.isFollowing == true
            toggleRow("Following its site", isOn: following, turnOn: "Follow", turnOff: "Unfollow") {
                await viewModel.setFollowed(!following)
            }
            let subscribed = post.isSubscribedComments == true
            toggleRow("Subscribed to comments", isOn: subscribed, turnOn: "Subscribe", turnOff: "Unsubscribe") {
                await viewModel.setCommentsSubscribed(!subscribed)
            }
            let notified = post.receivesCommentNotifications == true
            toggleRow("Comment notifications", isOn: notified, turnOn: "Turn on", turnOff: "Turn off") {
                await viewModel.setCommentNotifications(!notified)
            }
        }
    }

    /// A row that says whether something is on, with the button that changes it.
    private func toggleRow(
        _ title: String,
        isOn: Bool,
        turnOn: String,
        turnOff: String,
        action: @escaping () async -> Void
    ) -> some View {
        HStack {
            Text(title)
            Spacer()
            Text(isOn ? "Yes" : "No")
                .foregroundStyle(.secondary)
            Button(isOn ? turnOff : turnOn) {
                Task { await action() }
            }
            .buttonStyle(.borderless)
            .disabled(viewModel.isWorking)
        }
    }

    @ViewBuilder
    private func moreSection(_ post: ReaderPostRecord) -> some View {
        // Only a WordPress.com or Jetpack site's post has comments, likes and related posts.
        if let siteID = post.wordPressSiteID, let postID = Int64(exactly: post.identity.id) {
            Section("More") {
                NavigationLink("Comments", value: ReaderRoute.comments(siteID: siteID, postID: postID))
                NavigationLink("Who liked it", value: ReaderRoute.postLikes(siteID: siteID, postID: postID))
                NavigationLink(
                    "Related posts",
                    value: ReaderRoute.stream(.related(siteId: siteID, postId: postID))
                )
                NavigationLink("More from this site", value: ReaderRoute.stream(.site(siteId: siteID)))
            }
        } else if let feedID = post.identity.feedId {
            Section("More") {
                NavigationLink("More from this feed", value: ReaderRoute.stream(.feed(feedId: feedID)))
            }
        }
    }

    @ViewBuilder
    private func tagsSection(_ post: ReaderPostRecord) -> some View {
        if let tags = post.tags, !tags.isEmpty {
            Section("Tags") {
                ForEach(tags, id: \.slug) { tag in
                    NavigationLink(
                        (tag.displayName ?? tag.name).decodingHTMLEntities(),
                        value: ReaderRoute.stream(.tag(slug: tag.slug))
                    )
                }
            }
        }
    }

    private func contentSection(_ post: ReaderPostRecord) -> some View {
        Section("Content") {
            if let content = post.content, !content.isEmpty {
                let text = content.readerPlainText
                Text(text.count > 600 ? String(text.prefix(600)) + "…" : text)
                    .font(.callout)
                NavigationLink("Read the whole post", value: ReaderRoute.content(key: post.key))
            } else {
                Text("The cache has no content for this post yet.")
                    .foregroundStyle(.secondary)
            }
        }
    }
}

/// The whole of a post's content, as the cache has it.
struct ReaderPostContentView: View {
    let session: ReaderSession
    let key: ReaderSavedKey

    @State
    private var post: ReaderPostRecord?

    @State
    private var error: String?

    var body: some View {
        Group {
            if let post, let content = post.content {
                ReaderHTMLView(html: content, baseURL: post.identity.url.flatMap(URL.init(string:)))
            } else if let error {
                Text(error)
                    .foregroundStyle(.red)
                    .padding()
            } else {
                Text("The cache has no content for this post.")
                    .foregroundStyle(.secondary)
            }
        }
        .navigationTitle(post?.displayTitle ?? "Post")
        .task {
            do {
                post = try session.service.cachedPost(key: key)
            } catch {
                self.error = readerErrorMessage(error)
            }
        }
    }
}
