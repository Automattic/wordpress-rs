import SwiftUI
import WordPressAPI
import WordPressApiCache

/// The Reader tab. It works without a WordPress.com account, as the reader who isn't signed in, and as the
/// account when the app has one.
struct ReaderRootView: View {

    private enum Choice: Hashable {
        case account
        case signedOut
    }

    @EnvironmentObject
    private var loginManager: LoginManager

    @Environment(ReaderStore.self)
    private var store

    @State
    private var prefersSignedOut = false

    @State
    private var session: ReaderSession?

    @State
    private var loadError: String?

    @State
    private var path: [ReaderRoute] = []

    private var choice: Choice {
        loginManager.isLoggedInToWpCom && !prefersSignedOut ? .account : .signedOut
    }

    var body: some View {
        NavigationStack(path: $path) {
            Group {
                if let session {
                    ReaderHomeView(
                        viewModel: ReaderHomeViewModel(session: session),
                        path: $path,
                        readerPicker: loginManager.isLoggedInToWpCom ? $prefersSignedOut : nil
                    )
                    // A different reader is a different Reader, not this one with other values.
                    .id(session.owner)
                } else if let loadError {
                    ContentUnavailableView {
                        Text("The Reader couldn't open")
                    } description: {
                        Text(loadError)
                    } actions: {
                        Button("Try Again") {
                            Task { await loadSession() }
                        }
                        if choice == .account {
                            Button("Read Signed Out") {
                                prefersSignedOut = true
                            }
                        }
                    }
                } else {
                    ProgressView()
                }
            }
            .navigationTitle("Reader")
            .navigationDestination(for: ReaderRoute.self) { route in
                if let session {
                    destination(for: route, session: session)
                }
            }
        }
        .environment(\.openReaderRoute, openRoute)
        .task(id: choice) {
            await loadSession()
        }
    }

    private var openRoute: ReaderOpenAction {
        let path = $path
        return ReaderOpenAction { route in
            path.wrappedValue.append(route)
        }
    }

    private func loadSession() async {
        path = []
        session = nil
        loadError = nil
        do {
            switch choice {
            case .account: session = try await store.accountSession()
            case .signedOut: session = try store.signedOutSession()
            }
        } catch {
            loadError = readerErrorMessage(error)
        }
    }

    @ViewBuilder
    private func destination(for route: ReaderRoute, session: ReaderSession) -> some View {
        switch route {
        case .stream(let stream):
            ReaderStreamView(viewModel: ReaderStreamViewModel(session: session, stream: stream))
        case .post(let key, let identity):
            ReaderPostView(viewModel: ReaderPostViewModel(session: session, key: key, identity: identity))
        case .content(let key):
            ReaderPostContentView(session: session, key: key)
        case .comments(let siteID, let postID):
            ReaderCommentsView(viewModel: ReaderCommentsViewModel(session: session, siteID: siteID, postID: postID))
        case .postLikes(let siteID, let postID):
            ReaderLikesView(viewModel: ReaderLikesViewModel(session: session, subject: .post(siteID, postID)))
        case .commentLikes(let siteID, let commentID):
            ReaderLikesView(viewModel: ReaderLikesViewModel(session: session, subject: .comment(siteID, commentID)))
        case .saved:
            ReaderSavedPostsView(
                viewModel: ReaderSavedPostsViewModel { (session.service.saved(), session.cache) },
                opensPosts: true
            )
        case .offline:
            ReaderOfflineView(viewModel: ReaderOfflineViewModel(session: session))
        case .offlinePost(let key):
            ReaderOfflinePostView(session: session, key: key)
        case .subscriptions:
            ReaderSubscriptionsView(viewModel: ReaderSubscriptionsViewModel(session: session))
        case .blockedSites:
            ReaderBlockedSitesView(viewModel: ReaderBlockedSitesViewModel(session: session))
        case .findSites:
            ReaderFindSitesView(viewModel: ReaderFindSitesViewModel(session: session))
        }
    }
}
