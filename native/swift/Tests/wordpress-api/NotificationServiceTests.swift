import Foundation
import Testing
import WordPressAPI
import WordPressApiCache

/// Uses the notifications service the way an app does, through `WordPressAPI` and
/// `WordPressApiCache` only. None of these calls reach a server.
struct NotificationServiceTests {

    private let client = WPComApiClient(authentication: .none)

    /// The notifications service for account 1, made with its Reader service as an app makes it.
    private func makeService(
        _ client: WPComApiClient,
        cache: WordPressApiCache
    ) throws -> NotificationService {
        let reader = try client.makeReaderService(cache: cache, owner: .account(userId: 1), savedSource: .ios)
        return try client.makeNotificationService(reader: reader)
    }

    private func openCache() throws -> WordPressApiCache {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("notifications-\(UUID().uuidString).sqlite")
        let cache = try WordPressApiCache(path: url.path)
        _ = try cache.performMigrations()
        return cache
    }

    @Test
    func aNewAccountHasNoNotificationsYet() throws {
        let cache = try openCache()
        let service = try makeService(client, cache: cache)
        service.setLanguage(language: .portugueseBrazil)

        let notes: [WpComNotification] = try service.loadNotes(filter: NotificationsFilter())
        #expect(notes.isEmpty)
        let info: NotificationsInfo = try service.info()
        #expect(info.noteCount == 0)
        #expect(info.reachedEnd == false)
        #expect(info.lastSyncedAt == nil)
        #expect(service.language() == .portugueseBrazil)
        #expect(service.userId() == 1)
    }

    @Test
    func eachTabStartsEmptyAndHasItsName() async throws {
        let cache = try openCache()
        let service = try makeService(client, cache: cache)

        let tabs: [NotificationTab] = [.all, .unread, .comments, .subscribers, .likes]
        for tab in tabs {
            #expect(try service.loadTabNotes(tab: tab).isEmpty)
            #expect(try service.tabReachedEnd(tab: tab) == false)
        }
        #expect(
            tabs.map { $0.localizedText(languages: ["en-US"]) }
                == ["All", "Unread", "Comments", "Subscribers", "Likes"]
        )
        // Removing a note sends no request, so it works with no server at all.
        #expect(try await service.removeNote(noteId: 10_000_000_001) == false)
    }

    #if !os(Linux)
    /// Every request fails as it does on a device with no connection, so nothing reaches a server.
    @Test
    func changesMadeOfflineWaitAndShowAtOnce() async throws {
        let cache = try openCache()
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [NotificationsOfflineURLProtocol.self]
        let offline = WPComApiClient(
            urlSession: URLSession(configuration: configuration),
            authentication: .none
        )
        let service = try makeService(offline, cache: cache)

        let read: NotificationChangeOutcome = try await service.markRead(noteIds: [10_000_000_001])
        #expect(read == .waiting)
        #expect(try await service.markUnread(noteIds: [10_000_000_002]) == .waiting)
        let seen = Date(timeIntervalSince1970: 1_800_000_000)
        #expect(try await service.markSeen(time: seen) == .waiting)
        #expect(try await service.markSeen(time: seen) == .unchanged)
        // No note is cached, so there's no time to mark every note read up to.
        #expect(try await service.markAllRead(tab: .all, upTo: nil) == .unchanged)
        #expect(try service.pendingChangeCount() == 3)
        #expect(try await service.sendPendingChanges() == 3)
        #expect(try service.hasUnseenNotes() == false)
        // The seen time still waits, so the server isn't asked.
        #expect(try await service.fetchHasUnseenNotes() == false)
        #expect(try service.info().lastSeenTime == seen)
    }
    #endif

    @Test
    func aNoteThatIsNotCachedCannotBeLikedOrFollowed() async throws {
        let cache = try openCache()
        let service = try makeService(client, cache: cache)
        let noteID: NotificationId = 10_000_000_001

        try await expectNotCached(noteID) { try await service.setPostLiked(noteId: noteID, liked: true) }
        try await expectNotCached(noteID) { try await service.setCommentLiked(noteId: noteID, liked: false) }
        try await expectNotCached(noteID) {
            try await service.setSiteFollowed(noteId: noteID, siteId: 7001, followed: true)
        }
        #expect(NotificationServiceError.NoteNotCached(noteId: noteID).userMessage == nil)
        let refused = NotificationServiceError.ActionRefused(change: .followSite, error: .MissingIds)
        #expect(refused.userMessage(languages: ["en-US"]) == "You are not subscribed to this site.")
    }

    /// Expects `action` to fail because note `noteID` isn't cached.
    private func expectNotCached(
        _ noteID: NotificationId,
        _ action: () async throws -> ReaderActionOutcome
    ) async throws {
        do {
            _ = try await action()
            Issue.record("Expected the note not to be cached")
        } catch NotificationServiceError.NoteNotCached(let missing) {
            #expect(missing == noteID)
        }
    }

    @Test
    func signingOutRemovesTheAccountsNotifications() throws {
        let cache = try openCache()
        _ = try makeService(client, cache: cache)
        #expect(try cache.removeWordpressComAccount(userId: 1))
        #expect(try cache.removeWordpressComAccount(userId: 1) == false)
    }
}

#if !os(Linux)
/// Fails every request as a device without a connection does, so nothing reaches the network.
private final class NotificationsOfflineURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }

    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet))
    }

    override func stopLoading() {}
}
#endif
