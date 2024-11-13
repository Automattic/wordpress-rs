import SwiftUI
import WordPressAPI
import WordPressAPICombine
import Combine

@main
struct ExampleApp: App {

    @StateObject
    var loginManager = LoginManager()

    let rootListItems = [
        RootListData(name: "Application Passwords", callback: {
            try await WordPressAPI.globalInstance.applicationPasswords.listWithEditContext(userId: 1)
                .data
                .map { $0.asListViewData }
        }),
        RootListData(name: "Users", callback: {
            try await WordPressAPI.globalInstance.users.paginatedWithEditContext(params: UserListParams(perPage: 100))
                .map { $0.asListViewData }
        }),
        RootListData(name: "Plugins", callback: {
            try await WordPressAPI.globalInstance.plugins.listWithEditContext(params: .init())
                .data
                .map { $0.asListViewData }
        }),
        RootListData(name: "Post Types", callback: {
            try await WordPressAPI.globalInstance.postTypes.listWithViewContext().data.postTypes.map { _, value in
                value.asListViewData
            }
        }),
        RootListData(name: "Posts with Combine", stream: {
            let stream = try WordPressAPI.globalInstance.posts.paginatedStream(params: PostListParams(perPage: 5))

            return ListViewDataStream(
                publisher: stream.getPublisher().map { $0.asListViewData() }.eraseToAnyPublisher(),
                underlyingStream: stream
            )
        }),
        RootListData(name: "Posts with AsyncSequence", sequence: {
            do {
                let sequence = try WordPressAPI.globalInstance
                    .posts
                    .paginatedSequenceWithEditContext(params: PostListParams(perPage: 5))
                return ListViewSequence(underlyingSequence: sequence)
            } catch {
                abort()
            }
        }),
        RootListData(name: "Site Health Tests", callback: {
            let items: [any ListViewDataConvertable] = [
                try await WordPressAPI.globalInstance.siteHealthTests.authorizationHeader().data,
                try await WordPressAPI.globalInstance.siteHealthTests.backgroundUpdates().data,
                try await WordPressAPI.globalInstance.siteHealthTests.directorySizes().data,
                try await WordPressAPI.globalInstance.siteHealthTests.dotorgCommunication().data,
                try await WordPressAPI.globalInstance.siteHealthTests.httpsStatus().data,
                try await WordPressAPI.globalInstance.siteHealthTests.loopbackRequests().data,
                try await WordPressAPI.globalInstance.siteHealthTests.pageCache().data
            ]

            return items.map { $0.asListViewData }
        }),
        RootListData(name: "Site Settings", callback: {
            return try await WordPressAPI.globalInstance.siteSettings.retrieveWithEditContext().data.asListViewDataItems
        })
    ]

    var body: some Scene {
        WindowGroup {
            if loginManager.isLoggedIn {
                NavigationView {
                    // The first column is the sidebar.
                    RootListView(items: rootListItems)

                    // Initial content of the second column.
                    EmptyView()

                    // Initial content for the third column.
                    Text("Select a category of settings in the sidebar.")
                }.toolbar(content: {
                    #if os(macOS)
                    ToolbarItem {
                        Button("Log Out") {
                            Task {
                                await loginManager.logout()
                            }
                        }
                    }
                    #else
                    ToolbarItem(placement: .bottomBar) {
                        Button("Log Out") {
                            Task {
                                await loginManager.logout()
                            }
                        }
                    }
                    #endif
                })
            } else {
                LoginView()
            }
        }
        .environmentObject(loginManager)
    }
}
