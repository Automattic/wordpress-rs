import Foundation
import Testing
import WordPressApiCache

actor Test {

    private var cache: WordPressApiCache!
    private var changeCount = 0

    init() throws {
        self.cache = try WordPressApiCache()
    }

    @Test func testMigrationsWork() async throws {
        let migrationsPerformed = try await self.cache.performMigrations()
        #expect(migrationsPerformed == 5)
    }

    #if !os(Linux)
    @Test func testBackgroundUpdateNotificationsWork() async throws {
        let name = WordPressApiCache.Notifications.name(for: "_migrations")

        let handle = Task {
            for await _ in NotificationCenter.default.notifications(named: name) {
                self.incrementChangeCount()
            }
        }

        await self.cache.startListeningForUpdates()
        let migrationCount = try await self.cache.performMigrations()

        // Wait for NotificationCenter to finish delivery
        try await Task.sleep(nanoseconds: 10 * NSEC_PER_MSEC)

        #expect(migrationCount == self.changeCount)
        handle.cancel()
    }
    #endif

    func incrementChangeCount() {
        self.changeCount += 1
    }
}
