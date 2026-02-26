import SwiftUI
import WordPressAPI
import WordPressAPIInternal
import WordPressApiCache
import Combine

@main
struct ExampleApp: App {

    // swiftlint:disable:next force_try
    private let cache = try! WpApiCache(path: nil)

    private let handle: Any

    private let loginManager: LoginManager
    private let selfHostedService: SelfHostedService
    private let wpcomService: WPComService

    init() {
        // swiftlint:disable:next force_try
        _ = try! cache.performMigrations()
        let mockService = MockPostService(
            cache: cache,
            siteUrl: "https://vanilla.wpmt.co",
            apiRoot: "https://vanilla.wpmt.co/wp-json"
        )

        let ids = mockService.generateAndInsertPosts(count: 10)

        self.handle = mockService.startComprehensiveStressTest(
            entityIds: ids,
            config: StressTestConfig(
                minDelayMs: 100,
                maxDelayMs: 2000,
                minBatchSize: 100,
                maxBatchSize: 1000,
                updateWeight: 50,
                deleteWeight: 25,
                insertWeight: 25
            )
        )

        // swiftlint:disable:next force_try
        self.loginManager = try! LoginManager()
        self.selfHostedService = SelfHostedService(loginManager: self.loginManager)
        self.wpcomService = WPComService(loginManager: self.loginManager)
    }

    var body: some Scene {
        WindowGroup {
            TabView {
                Tab("Self-Hosted Site", systemImage: "server.rack") {
                    SelfHostedRootView()
                }

                Tab("WordPress.com", systemImage: "network") {
                    WPComRootView()
                }
            }
        }
        .environmentObject(self.loginManager)
        .environmentObject(self.selfHostedService)
        .environmentObject(self.wpcomService)
    }

    var toolbarItemPlacement: ToolbarItemPlacement {
        #if os(macOS)
        .automatic
        #else
        .bottomBar
        #endif
    }
}

extension Collection where Self.Element: Equatable {
    func contains(allOf elements: [Element]) -> Bool {
        elements.allSatisfy { element in
            self.contains(element)
        }
    }
}
