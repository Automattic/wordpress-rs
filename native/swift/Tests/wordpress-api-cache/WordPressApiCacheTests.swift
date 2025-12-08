import Foundation
import Testing
import WordPressAPI
import WordPressApiCache
import WordPressAPIInternal

actor Test {

    private var cache: WordPressApiCache
    private var changeCount = 0
    private let executor = WpRequestExecutor(urlSession: .shared)

    init() throws {
        self.cache = try WordPressApiCache()
    }

    @Test func testMigrationsWork() async throws {
        let migrationsPerformed = try await self.cache.performMigrations()
        #expect(migrationsPerformed == 6)
    }

    #if !os(Linux)
    @Test func testBackgroundUpdateNotificationsWork() async throws {

        let cache = try WpApiCache(path: ":memory:")
        _ = try cache.performMigrations()
        cache.startListeningForUpdates(delegate: DatabaseChangeNotifier.shared)

        let mockService = MockPostService(
            cache: cache,
            siteUrl: "https://vanilla.wpmt.co",
            apiRoot: "https://vanilla.wpmt.co/wp-json"
        )

        let delegate = WpApiClientDelegate(
            authProvider: .none(),
            requestExecutor: executor,
            middlewarePipeline: MiddlewarePipeline(middlewares: []),
            appNotifier: MockAppNotifier()
        )

        let apiUrl = try ParsedUrl.parse(input: "https://content-heavy.wpmt.co/wp-json")
        let service = try WpSelfHostedService(
            siteUrl: "https://content-heavy.wpmt.co",
            apiRoot: "https://content-heavy.wpmt.co/wp-json",
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: apiUrl),
            delegate: delegate,
            cache: cache
        )

        let publishedPosts = service.posts().getAllPostsWithEditContext()

//        DatabaseChangeNotifier.shared.startObserving(publishedPosts) { hook in
//            debugPrint("Published Posts changed: \(hook.table) \(hook.rowId) \(hook.action)")
//        }

        let ids = mockService.generateAndInsertPosts(count: 10_000)

        try await withThrowingTaskGroup { group in

            for _ in 0...10 {
                group.addTask {
                    _ = mockService.startComprehensiveStressTest(
                        entityIds: ids,
                        minDelayMs: 1,
                        maxDelayMs: 1000,
                        minBatchSize: 100,
                        maxBatchSize: 1000
                    )
                }
            }

            group.addTask {
                if #available(macOS 15.0, *) {
                    for try await values in DatabaseChangeNotifier.shared.startObserving(publishedPosts).map({ _ in
                        try await publishedPosts.loadData()
                    }) {
                        print("Received update hook: \(values.count)")
                    }
                } else {
                    // Fallback on earlier versions
                }
            }

//            group.addTask {
//                while(!Task.isCancelled) {
//                    try await Task.sleep(for: .seconds(1))
//                    let count = try await publishedPosts.loadData().count
//                    debugPrint(count)
//                }
//            }

            group.addTask {
                try await Task.sleep(for: .seconds(10))
                debugPrint("About to stop observing collection")
                DatabaseChangeNotifier.shared.stopObserving(publishedPosts)
            }

            try await Task {
                try await Task.sleep(for: .seconds(90))
            }.value

            group.cancelAll()
        }

        debugPrint("Done!")
    }
    #endif

    func incrementChangeCount() {
        self.changeCount += 1
    }
}

final class MockAppNotifier: WpAppNotifier {
    func requestedWithInvalidAuthentication(requestUrl: String) async {
        // no-op
    }
}
