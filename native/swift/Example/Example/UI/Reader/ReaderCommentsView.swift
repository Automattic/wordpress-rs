import SwiftUI
import WordPressAPI
import WordPressApiCache

/// A comment as the screen lists it: under the comment it answers, and indented by how deep the answer is.
struct ReaderCommentRow: Identifiable, Sendable {
    let comment: ReaderCommentRecord
    let depth: Int
    /// The comment answers one the cache doesn't have yet, so it's listed on its own.
    let parentIsMissing: Bool

    var id: String {
        if let commentId = comment.commentId {
            return "comment-\(commentId)"
        }
        return comment.localId ?? "comment-without-an-id"
    }

    /// The post's comments in the order to list them: each reply after the comment it answers.
    static func thread(_ comments: [ReaderCommentRecord]) -> [ReaderCommentRow] {
        let ids = Set(comments.compactMap(\.commentId))
        var replies: [Int64: [ReaderCommentRecord]] = [:]
        var top: [ReaderCommentRecord] = []
        for comment in comments {
            if let parent = comment.parentId, ids.contains(parent), parent != comment.commentId {
                replies[parent, default: []].append(comment)
            } else {
                top.append(comment)
            }
        }

        var rows: [ReaderCommentRow] = []
        var listed: Set<Int64> = []
        func add(_ comment: ReaderCommentRecord, depth: Int) {
            if let commentId = comment.commentId {
                // A comment is listed once, whatever the replies say about who answers whom.
                guard listed.insert(commentId).inserted else { return }
            }
            let parentIsMissing = depth == 0 && comment.parentId != nil && comment.parentId != 0
            rows.append(ReaderCommentRow(comment: comment, depth: depth, parentIsMissing: parentIsMissing))
            for reply in comment.commentId.flatMap({ replies[$0] }) ?? [] {
                add(reply, depth: depth + 1)
            }
        }
        for comment in top {
            add(comment, depth: 0)
        }
        return rows
    }
}

/// A post's comments from the cache, through the service's collection. The rows reload whenever the cache changes
/// in a way the comments show.
@MainActor
@Observable
final class ReaderCommentsViewModel {
    let session: ReaderSession
    let siteID: UInt64
    let postID: Int64

    @ObservationIgnored private let collection: ReaderCommentsCollection
    @ObservationIgnored private var watcher: ReaderCacheWatcher?
    @ObservationIgnored private var hasStarted = false
    @ObservationIgnored private var reloads = 0

    private(set) var rows: [ReaderCommentRow] = []
    private(set) var info: ReaderCommentsInfo?
    private(set) var replyingTo: ReaderCommentRecord?
    private(set) var notice: String?
    private var working = 0

    var draft = ""
    /// What went wrong loading the comments, shown in their status.
    var loadError: String?
    /// What went wrong with something the reader did, shown as an alert.
    var actionError: String?

    init(session: ReaderSession, siteID: UInt64, postID: Int64) {
        self.session = session
        self.siteID = siteID
        self.postID = postID
        self.collection = session.service.comments(siteId: siteID, postId: postID)
    }

    var isWorking: Bool { working > 0 }

    /// Shows what the cache has straight away, then fetches the newest comments.
    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        let collection = collection
        watcher = ReaderCacheWatcher(
            session.cache.databaseUpdatesPublisher()
                .filter { @Sendable hook in collection.isRelevantUpdate(hook: hook) }
        ) { [weak self] in
            await self?.reloadFromCache()
        }
        await reloadFromCache()
        await refresh()
    }

    func refresh() async {
        await load { try await self.collection.refresh() }
    }

    func loadMore() async {
        await load { try await self.collection.loadMore() }
    }

    func reply(to comment: ReaderCommentRecord?) {
        replyingTo = comment
    }

    /// Sends what's in the field, as a comment on the post or a reply to the comment picked.
    func send() async {
        let content = draft
        let parent = replyingTo?.commentId
        // With an account, the comment is kept in the list whether or not the server takes it, so the field
        // is emptied either way. Signed out it's refused and nothing keeps it, so the field does.
        if !session.isSignedOut {
            draft = ""
            replyingTo = nil
        }
        await act("Send") {
            _ = try await self.collection.postComment(content: content, parent: parent)
            return "The server took the comment."
        }
    }

    func sendAgain(_ comment: ReaderCommentRecord) async {
        guard let localID = comment.localId else { return }
        await act("Send again") {
            _ = try await self.collection.retryComment(localId: localID)
            return "The server took the comment."
        }
    }

    func discard(_ comment: ReaderCommentRecord) async {
        guard let localID = comment.localId else { return }
        await act("Discard") {
            try self.collection.discardComment(localId: localID)
                ? "The comment is gone."
                : "It wasn't discarded: it's being sent, or it's already gone."
        }
    }

    func setLiked(_ comment: ReaderCommentRecord, _ liked: Bool) async {
        guard let commentID = comment.commentId else { return }
        await act(liked ? "Like" : "Unlike") {
            try await self.session.service
                .setCommentLiked(siteId: self.siteID, commentId: commentID, liked: liked)
                .message
        }
    }

    private func load(_ work: () async throws -> ReaderCommentsPageResult) async {
        working += 1
        defer { working -= 1 }
        do {
            let page = try await work()
            loadError = nil
            notice = "Loaded \(readerCount(page.commentCount, "comment"))."
        } catch {
            loadError = readerErrorMessage(error)
        }
        await reloadFromCache()
    }

    private func act(_ name: String, _ work: () async throws -> String) async {
        working += 1
        defer { working -= 1 }
        do {
            notice = "\(name): \(try await work())"
        } catch {
            actionError = "\(name) failed. \(readerErrorMessage(error))"
        }
        await reloadFromCache()
    }

    private func reloadFromCache() async {
        reloads += 1
        let reload = reloads
        do {
            let comments = try await collection.loadItems()
            // A later reload has newer rows, and may already have shown them.
            guard reload == reloads else { return }
            rows = ReaderCommentRow.thread(comments)
            info = try collection.info()
        } catch {
            loadError = readerErrorMessage(error)
        }
    }
}

/// A post's comments, with replies under the comment they answer, and a field to write one.
struct ReaderCommentsView: View {

    @State
    var viewModel: ReaderCommentsViewModel

    var body: some View {
        List {
            statusSection
            composerSection
            Section(readerCount(viewModel.rows.count, "comment")) {
                ForEach(viewModel.rows) { row in
                    ReaderCommentRowView(row: row, viewModel: viewModel)
                }
                if viewModel.info?.hasMore == true {
                    Button("Load more") {
                        Task { await viewModel.loadMore() }
                    }
                    .disabled(viewModel.isWorking)
                    .accessibilityIdentifier("comments-load-more")
                }
            }
        }
        .navigationTitle("Comments")
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

    private var statusSection: some View {
        Section("Status") {
            HStack {
                Text(viewModel.info?.state.label ?? "Not loaded")
                if viewModel.isWorking {
                    ProgressView()
                }
            }
            LabeledContent("On the server", value: viewModel.info?.found.map { "\($0)" } ?? "Unknown")
            LabeledContent(
                "Last refreshed",
                value: viewModel.info?.lastRefreshedAt?.readerLabel ?? "Never"
            )
            if let error = viewModel.loadError ?? viewModel.info?.errorMessage {
                Text("Last error: \(error)")
                    .foregroundStyle(.red)
                    .accessibilityIdentifier("comments-error")
            }
            if let notice = viewModel.notice {
                ReaderNoticeRow(message: notice)
            }
        }
    }

    private var composerSection: some View {
        Section {
            if let comment = viewModel.replyingTo {
                HStack {
                    Text("Replying to \(comment.authorName?.decodingHTMLEntities() ?? "a comment")")
                    Spacer()
                    Button("Cancel") { viewModel.reply(to: nil) }
                        .buttonStyle(.borderless)
                }
            }
            TextField("Write a comment", text: $viewModel.draft, axis: .vertical)
                .accessibilityIdentifier("comment-draft")
            Button(viewModel.replyingTo == nil ? "Send comment" : "Send reply") {
                Task { await viewModel.send() }
            }
            .disabled(viewModel.draft.isEmpty || viewModel.isWorking)
            .accessibilityIdentifier("comment-send")
        } header: {
            Text("Write")
        } footer: {
            if viewModel.session.isSignedOut {
                Text("Writing a comment needs a WordPress.com account. Signed out, sending says so.")
            }
        }
    }
}

private struct ReaderCommentRowView: View {
    let row: ReaderCommentRow
    let viewModel: ReaderCommentsViewModel

    @Environment(\.openReaderRoute)
    private var open

    private var comment: ReaderCommentRecord { row.comment }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(byline)
                .font(.caption)
                .foregroundStyle(.secondary)
            if row.parentIsMissing {
                Text("A reply to a comment that hasn't loaded")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Text(comment.content?.readerPlainText ?? "")
            switch comment.sendState {
            case .sent: sentControls
            case .sending: ReaderBadge("Being sent")
            case .failed: failedControls
            }
        }
        .padding(.leading, CGFloat(min(row.depth, 6)) * 16)
    }

    private var byline: String {
        [authorName, comment.date?.readerLabel]
            .compactMap { $0 }
            .joined(separator: " · ")
    }

    /// A pingback's author is the title of the page that links here, which can carry character references.
    private var authorName: String {
        comment.authorName?.decodingHTMLEntities() ?? (comment.localId == nil ? "Someone" : "You")
    }

    @ViewBuilder
    private var sentControls: some View {
        if let commentID = comment.commentId {
            HStack(spacing: 16) {
                let liked = comment.iLike == true
                Text(readerCount(comment.likeCount ?? 0, "like"))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if liked {
                    ReaderBadge("Liked")
                }
                Button(liked ? "Unlike" : "Like") {
                    Task { await viewModel.setLiked(comment, !liked) }
                }
                Button("Reply") {
                    viewModel.reply(to: comment)
                }
                // A button, not a link: a link in a row makes the whole row open it.
                Button("Likers") {
                    open(.commentLikes(siteID: comment.siteId, commentID: commentID))
                }
            }
            .font(.callout)
            .buttonStyle(.borderless)
        }
    }

    private var failedControls: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Not taken: \(comment.sendError ?? "no reason was kept")")
                .font(.caption)
                .foregroundStyle(.red)
            HStack(spacing: 16) {
                Button("Send again") {
                    Task { await viewModel.sendAgain(comment) }
                }
                Button("Discard", role: .destructive) {
                    Task { await viewModel.discard(comment) }
                }
            }
            .buttonStyle(.borderless)
            .disabled(viewModel.isWorking)
        }
    }
}
