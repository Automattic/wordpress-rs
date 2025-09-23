import Foundation
import Testing
import WordPressApiCache

struct Test {

    private var cache: WordPressApiCache!

    init() throws {
        self.cache = try WordPressApiCache()
    }

    @Test func testMigrationsWork() async throws {
        let migrationsPerformed = try await self.cache.performMigrations()
        #expect(migrationsPerformed == 2)
    }

    @Test func testBackgroundUpdateNotificationsWork() async throws {
        var migrationNotificationReceived = 0
        NotificationCenter.default.addObserver(forName: WordPressApiCache.Notifications.name(for: "_migrations"), object: nil, queue: nil, using: { notification in
            migrationNotificationReceived += 1
        })
        await self.cache.startListeningForUpdates()
        _ = try await self.cache.performMigrations()
        #expect(migrationNotificationReceived == 2)
    }
}
