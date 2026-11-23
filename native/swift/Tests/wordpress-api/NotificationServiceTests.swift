import Foundation
import Testing
import WordPressAPI
import WordPressApiCache

/// Uses the notifications service the way an app does, through `WordPressAPI` and
/// `WordPressApiCache` only. None of these calls touch the network.
struct NotificationServiceTests {

    private let client = WPComApiClient(authentication: .none)

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
        let service = try client.makeNotificationService(cache: cache, userID: 1)
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
    func signingOutRemovesTheAccountsNotifications() throws {
        let cache = try openCache()
        _ = try client.makeNotificationService(cache: cache, userID: 1)
        #expect(try cache.removeWordpressComAccount(userId: 1))
        #expect(try cache.removeWordpressComAccount(userId: 1) == false)
    }
}
