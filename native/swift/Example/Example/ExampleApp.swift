import SwiftUI
import WordPressAPI

@main
struct ExampleApp: App {

    @StateObject
    var loginManager = LoginManager()

    let rootListItems = [
        RootListData(name: "Application Passwords", callback: {
            try await WordPressAPI.globalInstance.applicationPasswords.listWithEditContext(userId: 1)
                .map { $0.asListViewData }
        }),
        RootListData(name: "Users", callback: {
            try await WordPressAPI.globalInstance.users.listWithEditContext(params: .init())
                .map { $0.asListViewData }
        }),
        RootListData(name: "Plugins", callback: {
            try await WordPressAPI.globalInstance.plugins.listWithEditContext(params: .init())
                .map { $0.asListViewData }
        }),
        RootListData(name: "Post Types", callback: {
            let postTypeList = try await WordPressAPI.globalInstance.postTypes.listWithViewContext()
            return [
                postTypeList.post.asListViewData,
                postTypeList.page.asListViewData,
                postTypeList.attachment.asListViewData,
                postTypeList.navMenuItem.asListViewData,
                postTypeList.wpBlock.asListViewData,
                postTypeList.wpTemplate.asListViewData,
                postTypeList.wpTemplatePart.asListViewData,
                postTypeList.wpNavigation.asListViewData,
                postTypeList.wpFontFamily.asListViewData,
                postTypeList.wpFontFace.asListViewData
            ]
        }),
        RootListData(name: "Site Health Tests", callback: {
            return try await [
                WordPressAPI.globalInstance.siteHealthTests.authorizationHeader(),
                WordPressAPI.globalInstance.siteHealthTests.httpsStatus(),
                WordPressAPI.globalInstance.siteHealthTests.dotorgCommunication(),
                WordPressAPI.globalInstance.siteHealthTests.backgroundUpdates(),
                WordPressAPI.globalInstance.siteHealthTests.loopbackRequests()
            ].map { $0.asListViewData }
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
