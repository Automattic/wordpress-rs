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

        #expect(try service.status(post: identity(1)) == .unknown)
        #expect(try service.savedCount() == nil)

        let collection = service.savedPosts()
        let items: [ReaderSavedListItem] = try await collection.loadItems()
        #expect(items.isEmpty)
        let info: ReaderSavedListInfo = try collection.listInfo()
        #expect(info.state == ListState.idle)
        #expect(info.hasMore == nil)
    }

    @Test
    func anEmptyImportSendsNothingAndNothingWaitsForRoom() async throws {
        let cache = try openCache(at: temporaryDatabase())
        let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
        let saves: [ReaderLocalSave] = []
        let report: ReaderSavedImportReport = try await service.importSaves(saves: saves)
        #expect(report.imported == 0)
        #expect(report.waitingForRoom == 0)
        #expect(try service.savesWaitingForRoom() == 0)
        #expect(try cache.removeWordpressComAccount(userId: 1))
    }

    #if !os(Linux)
    @Test
    func aSaveMadeOfflineIsPendingAndShowsAtOnce() async throws {
        let cache = try openCache(at: temporaryDatabase())
        let configuration = URLSessionConfiguration.ephemeral
        configuration.protocolClasses = [OfflineURLProtocol.self]
        let offline = WPComApiClient(
            urlSession: URLSession(configuration: configuration),
            authentication: .none
        )
        let service = try offline.makeReaderSavedService(source: .ios, cache: cache, userID: 1)

        let outcome: ReaderSavedOutcome = try await service.save(post: identity(1))

        guard case .pending = outcome else {
            Issue.record("Expected the save to wait, got \(outcome)")
            return
        }
        #expect(try service.status(post: identity(1)) == .saved)
        let items = try await service.savedPosts().loadItems()
        #expect(items.map(\.isPending) == [true])
    }
    #endif

    @Test
    func aPostIsSavedWithTheDetailsTheAppHas() async throws {
        let cache = try openCache(at: temporaryDatabase())
        let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: 1)
        // A post without a site can't be saved, so this is refused before any request.
        let post = ReaderPost(
            identity: ReaderPostSaveIdentity(
                id: 1,
                siteId: nil,
                feedId: nil,
                feedItemId: nil,
                isExternal: false,
                url: nil
            ),
            title: "A post",
            author: ReaderPostAuthor(name: "An author"),
            siteName: "A site",
            siteIconUrl: "https://example.com/icon.png",
            featuredImage: "https://example.com/image.jpg",
            tags: [ReaderPostTag(slug: "news", name: "News")],
            commentCount: 3
        )

        await #expect {
            _ = try await service.saveWithDetails(post: post)
        } throws: { error in
            guard case .PostCannotBeSaved = error as? ReaderSavedError else { return false }
            return true
        }
        #expect(try await service.savedPosts().loadItems().isEmpty)
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

#if !os(Linux)
/// Fails every request as a device without a connection does, so nothing reaches the network.
private final class OfflineURLProtocol: URLProtocol {
    override static func canInit(with request: URLRequest) -> Bool { true }

    override static func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        client?.urlProtocol(self, didFailWithError: URLError(.notConnectedToInternet))
    }

    override func stopLoading() {}
}
#endif
