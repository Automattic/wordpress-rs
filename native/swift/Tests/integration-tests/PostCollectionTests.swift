import Foundation
import Testing
@preconcurrency import Combine

@testable import WordPressAPI
@testable import WordPressApiCache
import WordPressAPIInternal

struct PostCollectionTests {
    let api = WordPressAPI.admin()

    /// Reproduces an issue where refreshing one post collection trigger updates on an unrelated collection.
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

    /// Reproduces an issue where refreshing a post collection sends way too many updates.
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
