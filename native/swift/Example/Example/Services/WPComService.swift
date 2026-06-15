import Foundation
import WordPressAPI
import WordPressApiCache
import SwiftUI

@MainActor
final class WPComService: ObservableObject {

    private let loginManager: LoginManager

    init(loginManager: LoginManager) {
        self.loginManager = loginManager
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
                            botId: "jetpack-chat-mobile",
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
