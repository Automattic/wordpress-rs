import Combine
import Foundation
import WordPressAPI
import WordPressApiCache

/// One reader's Reader: an account's, or the reader's who isn't signed in. Every Reader screen shares it.
struct ReaderSession: Sendable {
    let service: ReaderService
    let cache: WordPressApiCache
    let owner: ReaderOwner
    /// Who the reader is, in words.
    let name: String
    /// Kept for as long as the service is: the service sends its requests through it.
    let client: WPComApiClient

    var isSignedOut: Bool {
        owner == .signedOut
    }
}

enum ReaderStoreError: LocalizedError {
    case notSignedIn

    var errorDescription: String? {
        switch self {
        case .notSignedIn: "The app isn't signed in to WordPress.com."
        }
    }
}

/// Creates the Reader services: one for the reader who isn't signed in, and one for the WordPress.com account
/// when the app has one. Both keep what they know in one cache file, so it's still there after a relaunch, and
/// what was followed and saved signed out can be taken over by the account.
@MainActor
@Observable
final class ReaderStore {
    private static let userIDKey = "reader.account.user-id"
    private static let displayNameKey = "reader.account.display-name"

    @ObservationIgnored private let loginManager: LoginManager
    @ObservationIgnored private var cache: WordPressApiCache?
    @ObservationIgnored private var signedOut: ReaderSession?
    @ObservationIgnored private var account: Task<ReaderSession, Error>?
    @ObservationIgnored private var loginObserver: AnyCancellable?

    /// Whether opening a post reports it with `postOpened`, which may count a view. Off until it's switched on,
    /// each launch: a counted view can't be taken back.
    var countsPageViews = false

    /// Whether every Reader request fails as if the device had no connection.
    private(set) var isOffline = ReaderOfflineURLProtocol.isOffline

    /// What went wrong when the account's cached data was deleted at sign-out, if anything did.
    private(set) var signOutError: String?

    init(loginManager: LoginManager) {
        self.loginManager = loginManager
        // A sign-in or sign-out means whatever was known about the account is about another one.
        loginObserver = loginManager.$isLoggedInToWpCom
            .removeDuplicates()
            .dropFirst()
            .sink { [weak self] isLoggedIn in
                Task { @MainActor in self?.accountChanged(isLoggedIn: isLoggedIn) }
            }
    }

    func setOffline(_ offline: Bool) {
        ReaderOfflineURLProtocol.isOffline = offline
        isOffline = offline
    }

    /// The Reader of the reader who isn't signed in. It sends no authentication.
    func signedOutSession() throws -> ReaderSession {
        if let signedOut {
            return signedOut
        }
        let session = try makeSession(
            authentication: .none,
            owner: .signedOut,
            name: "Signed out (this device)"
        )
        signedOut = session
        return session
    }

    /// The WordPress.com account's Reader. The first time, this asks `/me` who the account is. After that it's
    /// remembered, as an app would have it, so the Reader opens offline.
    func accountSession() async throws -> ReaderSession {
        if let account {
            return try await account.value
        }
        let task = Task { try await self.makeAccountSession() }
        account = task
        do {
            return try await task.value
        } catch {
            // Don't remember a failure: the next call tries again.
            account = nil
            throw error
        }
    }

    private func makeAccountSession() async throws -> ReaderSession {
        guard let authentication = try loginManager.getWpComLoginCredentials() else {
            throw ReaderStoreError.notSignedIn
        }
        let userID: UInt64
        let displayName: String?
        if let stored = UserDefaults.standard.object(forKey: Self.userIDKey) as? UInt64 {
            userID = stored
            displayName = UserDefaults.standard.string(forKey: Self.displayNameKey)
        } else {
            let user = try await makeClient(authentication: authentication).me.get().data
            UserDefaults.standard.set(user.id, forKey: Self.userIDKey)
            UserDefaults.standard.set(user.displayName, forKey: Self.displayNameKey)
            userID = user.id
            displayName = user.displayName
        }
        return try makeSession(
            authentication: authentication,
            owner: .account(userId: userID),
            name: [displayName, "WordPress.com user \(userID)"].compactMap { $0 }.joined(separator: ", ")
        )
    }

    private func makeSession(
        authentication: WpAuthentication,
        owner: ReaderOwner,
        name: String
    ) throws -> ReaderSession {
        let cache = try sharedCache()
        let client = makeClient(authentication: authentication)
        let service = try client.makeReaderService(cache: cache, owner: owner, savedSource: .ios)
        return ReaderSession(service: service, cache: cache, owner: owner, name: name, client: client)
    }

    private func makeClient(authentication: WpAuthentication) -> WPComApiClient {
        WPComApiClient(
            urlSession: ReaderNetwork.urlSession,
            authentication: authentication,
            middlewarePipeline: MiddlewarePipeline(middlewares: [DebugMiddleware()])
        )
    }

    /// The one Reader cache. It's a file, so streams, posts, follows and saves are still there after a relaunch,
    /// before any request.
    private func sharedCache() throws -> WordPressApiCache {
        if let cache {
            return cache
        }
        let directory = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let cache = try WordPressApiCache(path: directory.appendingPathComponent("reader-saved-posts.sqlite").path)
        _ = try cache.performMigrations()
        cache.startListeningForUpdates()
        self.cache = cache
        return cache
    }

    private func accountChanged(isLoggedIn: Bool) {
        account = nil
        let userID = UserDefaults.standard.object(forKey: Self.userIDKey) as? UInt64
        UserDefaults.standard.removeObject(forKey: Self.userIDKey)
        UserDefaults.standard.removeObject(forKey: Self.displayNameKey)

        signOutError = nil
        guard !isLoggedIn, let userID else { return }
        // The account signed out, so what was cached for it goes.
        do {
            try sharedCache().removeWordpressComAccount(userId: userID)
        } catch {
            signOutError = readerErrorMessage(error)
        }
    }
}
