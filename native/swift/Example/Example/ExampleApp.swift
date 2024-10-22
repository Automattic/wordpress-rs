import SwiftUI
import WordPressAPI

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
            try await WordPressAPI.globalInstance.users.listWithEditContext(params: .init())
                .data
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
        RootListData(name: "Site Health Tests", callback: {
            let items: [any ListViewDataConvertable] = try await [
                WordPressAPI.globalInstance.siteHealthTests.authorizationHeader(),
                WordPressAPI.globalInstance.siteHealthTests.backgroundUpdates(),
                WordPressAPI.globalInstance.siteHealthTests.directorySizes(),
                WordPressAPI.globalInstance.siteHealthTests.dotorgCommunication(),
                WordPressAPI.globalInstance.siteHealthTests.httpsStatus(),
                WordPressAPI.globalInstance.siteHealthTests.loopbackRequests(),
                WordPressAPI.globalInstance.siteHealthTests.pageCache()
            ]

            return items.map { $0.data.asListViewData }
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
