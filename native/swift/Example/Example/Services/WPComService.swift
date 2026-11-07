import Foundation
import WordPressAPI
import WordPressApiCache
import SwiftUI

@MainActor
final class WPComService: ObservableObject {

    private let loginManager: LoginManager

    /// One per account, shared by every screen: each instance only knows its own writes.
    private var readerSaved: (service: ReaderSavedService, cache: WordPressApiCache)?

    init(loginManager: LoginManager) {
        self.loginManager = loginManager
    }

    /// The account's saved posts service, and the cache it keeps them in. The cache is a file, so
    /// the list is still there after a relaunch, before any refresh.
    func sharedReaderSavedService() async throws -> (service: ReaderSavedService, cache: WordPressApiCache) {
        if let readerSaved {
            return readerSaved
        }
        let client = try await WPComApiClient.instance(loginManager: self.loginManager)
        let userID = try await client.me.get().data.id

        let directory = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        )
        let cache = try WordPressApiCache(path: directory.appendingPathComponent("reader-saved-posts.sqlite").path)
        _ = try cache.performMigrations()
        cache.startListeningForUpdates()

        let service = try client.makeReaderSavedService(source: .ios, cache: cache, userID: userID)
        readerSaved = (service, cache)
        return (service, cache)
    }

    public func loadRootListItems() async throws -> [RootListData] {
        [
            RootListData(name: "Support Conversations", category: .system) {
                let client = try await WPComApiClient.instance(loginManager: self.loginManager)
                return try await client.supportTickets.getSupportConversationList().data
                    .map {
                        ListViewData(
                            id: String($0.id),
                            title: $0.title,
                            subtitle: $0.createdAt.formatted(),
                            fields: [:]
                        )
                    }
            },

            RootListData(name: "Unified Conversations", category: .system) {
                let client = try await WPComApiClient.instance(loginManager: self.loginManager)
                return try await client.unifiedConversations.getUnifiedConversationList().data
                    .map {
                        ListViewData(
                            id: String($0.id),
                            title: $0.title,
                            subtitle: "\($0.status) · \($0.createdAt.formatted())",
                            fields: [:]
                        )
                    }
            },

            RootListData(
                name: "Bot Conversations",
                category: .system,
                callback: {
                    try await WPComApiClient.instance(loginManager: self.loginManager).supportBots
                        .getBotConversationList(
                            botId: "wpcom-workflow-support_chat",
                            params: ListBotConversationParams()
                        )
                        .data
                        .map {
                            ListViewData(
                                id: String($0.chatId),
                                title: $0.summaryMessage.content,
                                subtitle: $0.createdAt.formatted(),
                                fields: [:]
                            )
                        }
                }
            ),

            RootListData.view(
                "Saved Posts",
                {
                    AnyView(
                        ReaderSavedPostsView(
                            viewModel: ReaderSavedPostsViewModel {
                                try await self.sharedReaderSavedService()
                            }
                        )
                    )
                },
                .posts
            ),

            // System
            RootListData(
                name: "Me",
                category: .system,
                callback: {
                    let data = try await WPComApiClient.instance(loginManager: self.loginManager).me.get().data

                    return [
                        ListViewData(key: "User ID", value: "\(data.id)"),
                        ListViewData(key: "Display Name", value: data.displayName),
                        ListViewData(key: "Email Address", value: data.email)
                    ]
                }
            )
        ]
    }
}
