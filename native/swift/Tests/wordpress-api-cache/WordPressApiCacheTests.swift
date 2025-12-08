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
        let migrationsPerformed = try self.cache.performMigrations()
        #expect(migrationsPerformed == 6)
    }

    #if !os(Linux)
    @Test func testBackgroundUpdateNotificationsWork() async throws {
        let name = WordPressApiCache.Notifications.name(for: "_migrations")

        let handle = Task {
            for await _ in NotificationCenter.default.notifications(named: name) {
                self.incrementChangeCount()
            }
        }

        // Wait for the observer Task to start running.
        try await Task.sleep(nanoseconds: 100 * NSEC_PER_MSEC)

        self.cache.startListeningForUpdates()
        let migrationCount = try self.cache.performMigrations()

        // Wait for NotificationCenter to finish delivery
        try await Task.sleep(nanoseconds: 100 * NSEC_PER_MSEC)

        #expect(migrationCount == self.changeCount)
        handle.cancel()
    }
    #endif

    func incrementChangeCount() {
        self.changeCount += 1
    }
}
