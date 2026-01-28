import Foundation
import Testing
@preconcurrency import Combine

@testable import WordPressAPI
@testable import WordPressApiCache
import WordPressAPIInternal

struct PostCollectionTests {
    let api = WordPressAPI.admin()

    /// Verifies that refreshing one post collection does not trigger updates on unrelated collections.
    ///
    /// Given two unrelated post collections—one for draft posts and one for published posts—this test
    /// ensures that refreshing one collection does not cause updates on the other collection.
    @Test
    func updateShouldBeIsolated() async throws {
        let (cache, service) = try testContext()

        let draftCollection = service
            .posts()
            .createPostMetadataCollectionWithEditContext(
                endpointType: .posts,
                filter: .init(status: [.draft]),
                perPage: 10
            )
        let draftCollectionUpdates: Task<[UpdateHook], Never> = Task {
            await cache.databaseUpdatesPublisher()
                .filter { [draftCollection] in draftCollection.isRelevantUpdate(hook: $0) }
                .timeout(1, scheduler: DispatchQueue.main)
                .values
                .reduce(into: []) { $0.append($1) }
        }

        let publishCollection = service
            .posts()
            .createPostMetadataCollectionWithEditContext(
                endpointType: .posts,
                filter: .init(status: [.publish]),
                perPage: 10
            )

        _ = try await publishCollection.refresh()

        await #expect(draftCollectionUpdates.value.count == 0)
    }

    @Test
    func minimalUpdates() async throws {
        let (cache, service) = try testContext()

        let collection = service
            .posts()
            .createPostMetadataCollectionWithEditContext(
                endpointType: .posts,
                filter: .init(status: [.draft]),
                perPage: 10
            )
        let updates: Task<[UpdateHook], Never> = Task {
            await cache.databaseUpdatesPublisher()
                .filter { [collection] in collection.isRelevantUpdate(hook: $0) }
                .timeout(1, scheduler: DispatchQueue.main)
                .values
                .reduce(into: []) { $0.append($1) }
        }

        _ = try await collection.refresh()

        // TODO: What's the reasonable amount of updates for the `refresh` call?
        await #expect(updates.value.count < 5)
    }

    private func testContext() throws -> (WordPressApiCache, WpSelfHostedService) {
        let cache: WordPressApiCache = try WordPressApiCache()
        _ = try cache.performMigrations()
        cache.startListeningForUpdates()

        return try (cache, api.createSelfHostedService(cache: cache))
    }
}
