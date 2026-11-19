import SwiftUI
import WordPressAPI

/// The users who liked a post or a comment. They aren't cached.
@MainActor
@Observable
final class ReaderLikesViewModel {
    enum Subject {
        case post(UInt64, Int64)
        case comment(UInt64, Int64)
    }

    /// The most likes the API returns in a page.
    private static let pageSize: UInt32 = 90

    let session: ReaderSession
    let subject: Subject

    @ObservationIgnored private var hasStarted = false

    private(set) var likes: [ReaderLike] = []
    private(set) var unreadable: [ReaderUnreadableLike] = []
    private(set) var found: UInt64?
    private(set) var iLike: Bool?
    private(set) var profile: ReaderUserProfile?
    private(set) var isWorking = false

    /// What went wrong fetching the likes, shown where the list would be.
    var loadError: String?
    /// What went wrong fetching a user's profile, shown as an alert.
    var actionError: String?

    init(session: ReaderSession, subject: Subject) {
        self.session = session
        self.subject = subject
    }

    func start() async {
        guard !hasStarted else { return }
        hasStarted = true
        await load()
    }

    func load() async {
        isWorking = true
        defer { isWorking = false }
        do {
            let params = ReaderLikesParams(number: Self.pageSize)
            let page =
                switch subject {
                case .post(let siteID, let postID):
                    try await session.service.postLikes(siteId: siteID, postId: postID, params: params)
                case .comment(let siteID, let commentID):
                    try await session.service.commentLikes(siteId: siteID, commentId: commentID, params: params)
                }
            likes = page.likes
            unreadable = page.unreadableLikes
            found = page.found
            iLike = page.iLike
            loadError = nil
        } catch {
            loadError = readerErrorMessage(error)
        }
    }

    /// Fetches a user's public profile, by their login when the like has it.
    func showProfile(of like: ReaderLike) async {
        isWorking = true
        defer { isWorking = false }
        do {
            let user: WpComUserIdentifier = like.login.map { .login(value: $0) } ?? .id(value: like.id)
            profile = try await session.service.userProfile(user: user)
        } catch {
            actionError = "The profile couldn't be fetched. \(readerErrorMessage(error))"
        }
    }

    func dismissProfile() {
        profile = nil
    }
}

struct ReaderLikesView: View {

    @State
    var viewModel: ReaderLikesViewModel

    var body: some View {
        List {
            Section("Status") {
                HStack {
                    Text(viewModel.isWorking ? "Working…" : "Idle")
                    if viewModel.isWorking {
                        ProgressView()
                    }
                }
                LabeledContent("Likes on the server", value: viewModel.found.map { "\($0)" } ?? "Unknown")
                if let iLike = viewModel.iLike {
                    LabeledContent("The reader is one of them", value: iLike ? "Yes" : "No")
                }
                if let error = viewModel.loadError {
                    Text("Last error: \(error)")
                        .foregroundStyle(.red)
                        .accessibilityIdentifier("likes-error")
                }
                if !viewModel.unreadable.isEmpty {
                    Text("\(viewModel.unreadable.count) likes on this page couldn't be read.")
                        .foregroundStyle(.red)
                }
            }

            if let profile = viewModel.profile {
                profileSection(profile)
            }

            Section {
                ForEach(viewModel.likes, id: \.id) { like in
                    Button {
                        Task { await viewModel.showProfile(of: like) }
                    } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(like.name ?? like.login ?? "User \(like.id)")
                            Text(
                                [like.login, like.dateLiked.map { "liked \($0.readerLabel)" }]
                                    .compactMap { $0 }
                                    .joined(separator: " · ")
                            )
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        }
                    }
                }
            } header: {
                Text("The newest \(viewModel.likes.count) likes")
            } footer: {
                Text("Tap a user for their public profile, which needs a WordPress.com account.")
            }
        }
        .navigationTitle("Likes")
        .refreshable { await viewModel.load() }
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

    private func profileSection(_ profile: ReaderUserProfile) -> some View {
        Section("Profile") {
            LabeledContent("Name", value: profile.displayName ?? "Unknown")
            LabeledContent("Login", value: profile.login ?? "Unknown")
            LabeledContent("User ID", value: "\(profile.id)")
            if let description = profile.description, !description.isEmpty {
                Text(description.readerPlainText)
            }
            if let address = profile.profileUrl {
                LabeledContent("Profile", value: address)
            }
            Button("Close the profile") {
                viewModel.dismissProfile()
            }
        }
    }
}
