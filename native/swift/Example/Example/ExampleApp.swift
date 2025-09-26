import SwiftUI
import WordPressAPI
import Combine

private let userListParams = UserListParams(perPage: 5)
private let postListParams = PostListParams(perPage: 5)
private let mediaListParams = MediaListParams(perPage: 5)

@main
struct ExampleApp: App {

    @StateObject
    var loginManager = LoginManager()

    @State
    var showUploadView = false

    @State
    var rootListItems: [RootListData] = []

    @State
    var isLoadingInitialData: Bool = true

    var body: some Scene {
        WindowGroup {
            if loginManager.isLoggedIn {
                NavigationView {
                    if isLoadingInitialData {
                        ProgressView()
                    } else {
                        // The first column is the sidebar.
                        RootListView(items: rootListItems)
                    }

                    // Initial content of the second column.
                    EmptyView()

                    // Initial content for the third column.
                    Text("Select a category of settings in the sidebar.")
                }
                .sheet(isPresented: $showUploadView) {
                    UploadView()
                }
                .toolbar(content: {
                    ToolbarItem(placement: toolbarItemPlacement) {
                        Button("Log Out") {
                            Task {
                                await loginManager.logout()
                            }
                        }
                    }
                    ToolbarItem(placement: toolbarItemPlacement) {
                        Button("Add Media File") {
                            showUploadView = true
                        }
                    }
                })
                .task {
                    do {
                        self.rootListItems = try await self.loadSiteTypes()
                        self.isLoadingInitialData = false
                    } catch {
                        debugPrint(error.localizedDescription)
                    }
                }
            } else {
                LoginView()
            }
        }
        .environmentObject(loginManager)
    }

    func loadSiteTypes() async throws -> [RootListData] {
        let api = try await WordPressAPI.globalInstance
        var baseData = [
            RootListData(name: "Application Passwords", callback: {
                try await api.applicationPasswords.listWithEditContext(userId: 1)
                    .data
                    .map { $0.asListViewData }
            }),
            RootListData(name: "Users", sequence: {
                let sequence = await api.users.sequenceWithEditContext(params: userListParams)
                return ListViewSequence(underlyingSequence: sequence)
            }),
            RootListData(name: "Plugins", callback: {
                try await api.plugins.listWithEditContext(params: .init())
                    .data
                    .map { $0.asListViewData }
            }),
            RootListData(name: "Post Types", callback: {
                try await api.postTypes.listWithEditContext().data.postTypes.map { _, value in
                    value.asListViewData
                }
            }),
            RootListData(name: "Media", sequence: {
                let sequence = await api.media.sequenceWithEditContext(params: mediaListParams)
                return ListViewSequence(underlyingSequence: sequence)
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
            RootListData(name: "Taxonomies", callback: {
                try await api.taxonomies
                    .listWithEditContext(params: TaxonomyListParams())
                    .data
                    .taxonomyTypes
                    .map { (_, value) in
                        value.asListViewData
                    }
            }),
            RootListData(name: "Site Settings", callback: {
                return try await api.siteSettings.retrieveWithEditContext().data.asListViewDataItems
            })
        ]

        let postTypes = try await api.postTypes.listWithEditContext().data
            .postTypes
            .map(\.value)
            .filter { $0.visibility.showInNavMenus }
            .filter { $0.supports.keys.contains(allOf: [.title, .author, .customFields]) }

        for type in postTypes {
            baseData.append(RootListData(name: type.name, sequence: {
                let sequence = try await WordPressAPI.globalInstance.posts.sequenceWithEditContext(
                    type: PostEndpointType.custom(type.restBase),
                    params: postListParams
                )

                return ListViewSequence(underlyingSequence: sequence)
            }))
        }

        baseData.append(RootListData(name: "Posts (Direct)", sequence: {
            let sequence = try await WordPressAPI.globalInstance.posts.sequenceWithEditContext(
                type: .posts,
                params: postListParams
            )

            return ListViewSequence(underlyingSequence: sequence)
        }))

        baseData.append(RootListData(name: "Pages (Direct)", sequence: {
            let sequence = try await WordPressAPI.globalInstance.posts.sequenceWithEditContext(
                type: .pages,
                params: postListParams
            )

            return ListViewSequence(underlyingSequence: sequence)
        }))

        baseData.append(RootListData(name: "Post Statuses", callback: {
            try await WordPressAPI.globalInstance.postStatuses.listWithEditContext()
                .data
                .postStatuses
                .map(\.value)
                .map(\.asListViewData)
        }))

        return baseData
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
