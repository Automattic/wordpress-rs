import Foundation
import Testing
import WordPressAPI
import WordPressApiCache
import WordPressAPIInternal

#if canImport(Combine)
import Combine
#endif

// Most of the test functions test against the Combine API `databaseUpdatesPublisher`,
// because the Combine observer is synchronous, which is much easier to work with in
// unit tests. Unlike the `AsyncSequence` API, the order of execution is much more
// predictable.
//
// `.serialized` is required because every test observes updates through the shared
// `NotificationCenter.default`. Notifications are filtered by the per-cache broadcaster
// object, but once a cache (and its broadcaster) is deallocated (see
// `afterCacheDeallocated`), that object filter goes stale and the observer starts
// matching every update notification. Running in parallel then lets one test's updates
// leak into another test's observer, which made `afterCacheDeallocated` flaky.
@Suite(.timeLimit(.minutes(5)), .serialized)
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

        // Remove the observer at the end (via the returned token): once the cache is
        // deallocated below, its broadcaster is gone and this observer would otherwise
        // linger on the shared `NotificationCenter.default` with a stale object filter,
        // matching unrelated caches' update notifications.
        var token: NSObjectProtocol?
        defer {
            if let token {
                NotificationCenter.default.removeObserver(token)
            }
        }

        await confirmation(expectedCount: 5) { confirmation in
            token = cache?.addDatabaseUpdatesObserver { _ in confirmation() }

            _ = mockService.generateAndInsertPosts(count: 5)

            // The changes below happen after the cache is deallocated and must not be
            // sent to the observer.
            cache = nil
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
        let (cache, mockService) = try testContext()

        await confirmation(expectedCount: 10) { confirmation in
            let cancellable = cache.databaseUpdatesPublisher()
                .sink { _ in
                    confirmation()
                }

            _ = mockService.generateAndInsertPosts(count: 10)

            cancellable.cancel()
        }
    }

    // When multiple observers listen to a single `WordPressApiCache` instance, all observers receive updates.
    @Test func multipleObservers() async throws {
        let (cache, mockService) = try testContext()

        // Starts multiple tasks to listen for database updates in the background.
        await confirmation(expectedCount: 30) { confirmation in
            var cancellables = Set<AnyCancellable>()
            cache.databaseUpdatesPublisher().sink { _ in confirmation() }.store(in: &cancellables)
            cache.databaseUpdatesPublisher().sink { _ in confirmation() }.store(in: &cancellables)
            cache.databaseUpdatesPublisher().sink { _ in confirmation() }.store(in: &cancellables)

            _ = mockService.generateAndInsertPosts(count: 10)
        }
    }

    // Each observer is only notified when its specific `WordPressApiCache` instance is updated.
    @Test func observingMultipleCaches() async throws {
        let (cache0, mockService0) = try testContext()
        let (cache1, mockService1) = try testContext()
        let (cache2, mockService2) = try testContext()

        await withTaskGroup { group in
            group.addTask {
                await confirmation(expectedCount: 3) { confirmation in
                    let cancellable = cache0.databaseUpdatesPublisher()
                        .sink { _ in
                            confirmation()
                        }

                    _ = mockService0.generateAndInsertPosts(count: 3)
                    cancellable.cancel()
                }
            }
            group.addTask {
                await confirmation(expectedCount: 6) { confirmation in
                    let cancellable = cache1.databaseUpdatesPublisher()
                        .sink { _ in
                            confirmation()
                        }

                    _ = mockService1.generateAndInsertPosts(count: 6)
                    cancellable.cancel()
                }
            }
            group.addTask {
                await confirmation(expectedCount: 9) { confirmation in
                    let cancellable = cache2.databaseUpdatesPublisher()
                        .sink { _ in
                            confirmation()
                        }

                    _ = mockService2.generateAndInsertPosts(count: 9)
                    cancellable.cancel()
                }
            }
        }
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
