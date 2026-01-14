import Foundation
import Testing
import WordPressAPI
import WordPressApiCache
import WordPressAPIInternal

@Suite(.timeLimit(.minutes(5)))
struct WordPressApiCacheTests {
    @Test func addDatabaseUpdatesObserver() async throws {
        let (cache, mockService) = try testContext()

        await confirmation(expectedCount: 10) { confirmation in
            _ = cache.addDatabaseUpdatesObserver { _ in
                confirmation()
            }

            _ = mockService.generateAndInsertPosts(count: 10)
        }
    }

    @Test func noUpdatesAfterStop() async throws {
        let (cache, mockService) = try testContext(listingForUpdates: false)

        await confirmation(expectedCount: 0) { confirmation in
            cache.startListeningForUpdates()
            _ = mockService.generateAndInsertPosts(count: 10)
            cache.stopListeningForUpdates()

            _ = cache.addDatabaseUpdatesObserver { _ in
                confirmation()
            }

            _ = mockService.generateAndInsertPosts(count: 20)
        }
    }

    @Test func observeBeforeStart() async throws {
        let (cache, mockService) = try testContext(listingForUpdates: false)

        await confirmation(expectedCount: 10) { confirmation in
            _ = cache.addDatabaseUpdatesObserver { _ in
                confirmation()
            }

            cache.startListeningForUpdates()

            _ = mockService.generateAndInsertPosts(count: 10)
        }
    }

    @Test func observeAfterResume() async throws {
        let (cache, mockService) = try testContext(listingForUpdates: false)

        await confirmation(expectedCount: 20) { confirmation in
            cache.startListeningForUpdates()
            _ = mockService.generateAndInsertPosts(count: 10)
            cache.stopListeningForUpdates()

            cache.startListeningForUpdates()
            _ = cache.addDatabaseUpdatesObserver { _ in
                confirmation()
            }

            _ = mockService.generateAndInsertPosts(count: 20)
        }
    }

    @Test func afterCacheDeallocated() async throws {
        var (cache, mockService): (WordPressApiCache?, MockPostService) = try testContext()

        try await confirmation(expectedCount: 5) { confirmation in
            _ = cache?.addDatabaseUpdatesObserver { _ in
                confirmation()
            }

            _ = mockService.generateAndInsertPosts(count: 5)

            // The changes below should not be sent to the observer.
            cache = nil
            try await Task.sleep(for: .seconds(1))
            _ = mockService.generateAndInsertPosts(count: 10)
        }
    }

    @Test func stressTest() async throws {
        let (_, mockService) = try testContext()

        let ids = mockService.generateAndInsertPosts(count: 10_000)

        await withThrowingTaskGroup { group in
            for _ in 0...10 {
                group.addTask {
                    _ = mockService.startComprehensiveStressTest(
                        entityIds: ids,
                        config: StressTestConfig(
                            minDelayMs: 1,
                            maxDelayMs: 1000,
                            minBatchSize: 100,
                            maxBatchSize: 1000,
                            updateWeight: 50,
                            deleteWeight: 25,
                            insertWeight: 25
                        )
                    )
                }
            }
        }
    }

    #if !os(Linux)
    @Test func updatesReceived() async throws {
        var (cache, mockService): (WordPressApiCache?, MockPostService) = try testContext()

        // Starts a Task to listen for database updates in the background. The Task completes when `cache`
        // is deallocated.
        let numberOfUpdates = Task { [unowned cache] in
            return await cache!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }

        // Allow the observer task to initialize. The one-second delay is intentionally long to reduce flaky
        // failures on CI.
        try await Task.sleep(for: .seconds(1))
        _ = mockService.generateAndInsertPosts(count: 10)

        cache = nil
        await #expect(numberOfUpdates.value == 10)
    }

    // When multiple observers listen to a single `WordPressApiCache` instance, all observers receive updates.
    @Test func multipleObservers() async throws {
        var (cache, mockService): (WordPressApiCache?, MockPostService) = try testContext()

        // Starts multiple tasks to listen for database updates in the background. The tasks complete when `cache`
        // is deallocated.
        let numberOfUpdates0 = Task { [unowned cache] in
            return await cache!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        let numberOfUpdates1 = Task { [unowned cache] in
            return await cache!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        let numberOfUpdates2 = Task { [unowned cache] in
            return await cache!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        try await Task.sleep(for: .seconds(1))
        _ = mockService.generateAndInsertPosts(count: 10)

        cache = nil
        await #expect(numberOfUpdates0.value == 10)
        await #expect(numberOfUpdates1.value == 10)
        await #expect(numberOfUpdates2.value == 10)
    }

    // Each observer is only notified when its specific `WordPressApiCache` instance is updated.
    @Test func observingMultipleCaches() async throws {
        var (cache0, mockService0): (WordPressApiCache?, MockPostService) = try testContext()
        var (cache1, mockService1): (WordPressApiCache?, MockPostService) = try testContext()
        var (cache2, mockService2): (WordPressApiCache?, MockPostService) = try testContext()

        // Starts multiple tasks to listen for database updates in the background. The tasks complete when `cache`
        // is deallocated.
        let numberOfUpdates0 = Task { [unowned cache0] in
            return await cache0!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        let numberOfUpdates1 = Task { [unowned cache1] in
            return await cache1!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        let numberOfUpdates2 = Task { [unowned cache2] in
            return await cache2!.databaseUpdatesPublisher().values.reduce(0) { counter, _ in counter + 1 }
        }
        try await Task.sleep(for: .seconds(1))
        _ = mockService0.generateAndInsertPosts(count: 3)
        _ = mockService1.generateAndInsertPosts(count: 6)
        _ = mockService2.generateAndInsertPosts(count: 9)

        cache0 = nil
        cache1 = nil
        cache2 = nil

        await #expect(numberOfUpdates0.value == 3)
        await #expect(numberOfUpdates1.value == 6)
        await #expect(numberOfUpdates2.value == 9)
    }
    #endif

    private func testContext(listingForUpdates: Bool = true) throws -> (WordPressApiCache, MockPostService) {
        let cache: WordPressApiCache = try WordPressApiCache()
        _ = try cache.performMigrations()

        if listingForUpdates {
            cache.startListeningForUpdates()
        }

        let siteURL = "https://\(UUID().uuidString).example.com"
        let mockService = MockPostService(
            cache: cache.cache,
            siteUrl: siteURL,
            apiRoot: "\(siteURL)/wp-json"
        )

        return (cache, mockService)
    }
}
