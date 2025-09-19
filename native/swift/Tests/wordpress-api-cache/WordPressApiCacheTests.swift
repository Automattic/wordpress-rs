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
}
