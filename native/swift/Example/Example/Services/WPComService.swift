import Foundation
import WordPressAPI
import WordPressApiCache

struct WPComService {

    public func loadRootListItems() async throws -> [RootListData] {
        [
            RootListData(name: "Support Conversations", category: .system) {
                try await WPComApiClient.globalInstance.supportTickets.getSupportConversationList().data.map {
                    ListViewData(
                        id: String($0.id),
                        title: $0.title,
                        subtitle: $0.createdAt.formatted(),
                        fields: [:]
                    )
                }
            },

            RootListData(name: "Bot Conversations", category: .system, callback: {
                try await WPComApiClient.globalInstance.supportBots.getBotConversationList(
                    botId: "jetpack-chat-mobile",
                    params: ListBotConversationParams()
                ).data.map {
                    ListViewData(
                        id: String($0.chatId),
                        title: $0.summaryMessage.content,
                        subtitle: $0.createdAt.formatted(),
                        fields: [:]
                    )
                }
            }),

            // System
            RootListData(name: "Me", category: .system, callback: {
                let data = try await WPComApiClient.globalInstance.me.get().data

                return [
                    ListViewData(key: "User ID", value: "\(data.id)"),
                    ListViewData(key: "Display Name", value: data.displayName),
                    ListViewData(key: "Email Address", value: data.email)
                ]
            })
        ]
    }
}
