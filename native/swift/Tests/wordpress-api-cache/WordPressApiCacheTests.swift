import Foundation
import Testing
import WordPressAPI
import WordPressAPIInternal
import WordPressApiCache

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

//        let mockService = MockPostService(
//            cache: cache,
//            siteUrl: "https://vanilla.wpmt.co",
//            apiRoot: "https://vanilla.wpmt.co/wp-json"
//        )
//
//        let ids = mockService.generateAndInsertPosts(count: 10_000)

        let delegate = WpApiClientDelegate(
            authProvider: .staticWithAuth(auth: WpAuthentication(username: "admin", password: "SpBJ ChT0 pcZT okaf 8l27 iE9d")),
            requestExecutor: executor,
            middlewarePipeline: WpApiMiddlewarePipeline(middlewares: []),
            appNotifier: MockAppNotifier()
        )
        let service = try WpSelfHostedService(
            siteUrl: "https://content-heavy.wpmt.co",
            apiRoot: "https://content-heavy.wpmt.co/wp-json",
            apiUrlResolver: WpOrgSiteApiUrlResolver(apiRootUrl: ParsedUrl.parse(input: "https://content-heavy.wpmt.co/wp-json")),
            delegate: delegate,
            cache: cache
        )

        let publishedPosts = service.posts().createPostCollectionWithEditContext(filter: AnyPostFilter(status: .publish))

        DatabaseChangeNotifier.shared.startObserving(publishedPosts) { hook in
            debugPrint("Published Posts changed: \(hook.table) \(hook.rowId) \(hook.action)")
        }

        try await withThrowingTaskGroup { group in

            for i in 1...224 {
                group.addTask {
                    while(!Task.isCancelled) {
                        let result = try await publishedPosts.fetchPage(page: UInt32(i), perPage: 1)
                        debugPrint("fetched page \(i)")
                    }
                }
            }

            group.addTask {
                while(!Task.isCancelled) {
                    try await Task.sleep(for: .seconds(1))
                    let count = try await publishedPosts.loadData().count
                    debugPrint(count)
                }
            }

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
