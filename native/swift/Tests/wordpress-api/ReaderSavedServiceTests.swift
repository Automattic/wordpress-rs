import Foundation
import Testing
import WordPressAPI
import WordPressApiCache

/// Uses the saved posts service the way an app does, through `WordPressAPI` and
/// `WordPressApiCache` only. None of these calls touch the network.
struct ReaderSavedServiceTests {

    private let client = WPComApiClient(authentication: .none)

    private func identity(_ postID: UInt64) -> ReaderPostSaveIdentity {
        ReaderPostSaveIdentity(
            id: postID,
            siteId: 123,
            feedId: nil,
            feedItemId: nil,
            isExternal: false,
            url: nil
        )
    }

    private func temporaryDatabase() -> URL {
        FileManager.default.temporaryDirectory
            .appendingPathComponent("reader-saved-\(UUID().uuidString).sqlite")
    }

    private func openCache(at url: URL) throws -> WordPressApiCache {
        let cache = try WordPressApiCache(path: url.path)
        _ = try cache.performMigrations()
        return cache
    }

    @Test
    func aNewAccountHasNothingSavedYet() async throws {
        let cache = try openCache(at: temporaryDatabase())
        let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)

        #expect(service.status(post: identity(1)) == .unknown)
        #expect(service.savedCount() == nil)
        #expect(service.recentSaves().isEmpty)
        #expect(service.localOnlyKeys().isEmpty)

        let collection = service.savedPosts()
        let items: [ReaderSavedListItem] = try await collection.loadItems()
        #expect(items.isEmpty)
        let info: ReaderSavedListInfo = try collection.listInfo()
        #expect(info.state == ListState.idle)
        #expect(info.hasMore == nil)
    }

    @Test
    func theImportIsRememberedAcrossLaunchesUntilSignOut() async throws {
        let url = temporaryDatabase()
        defer { try? FileManager.default.removeItem(at: url) }

        do {
            let cache = try openCache(at: url)
            let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
            let report: ReaderSavedImportReport = try await service.importLocalSaves(saves: [])
            #expect(!report.alreadyImported)
        }

        do {
            let cache = try openCache(at: url)
            let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
            let saves: [ReaderLocalSave] = []
            #expect(try await service.importLocalSaves(saves: saves).alreadyImported)

            #expect(try cache.removeWordpressComAccount(userId: 1))
            #expect(try !cache.removeWordpressComAccount(userId: 1))
        }

        let cache = try openCache(at: url)
        let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
        #expect(try await !service.importLocalSaves(saves: []).alreadyImported)
    }

    @Test
    func aCacheWithoutMigrationsIsADatabaseError() throws {
        let cache = try WordPressApiCache(path: temporaryDatabase().path)

        #expect {
            _ = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
        } throws: { error in
            guard case .Database = error as? ReaderSavedError else { return false }
            return true
        }
    }
}
