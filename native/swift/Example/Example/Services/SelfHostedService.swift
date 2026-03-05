import Foundation
import WordPressAPI
import WordPressApiCache

@MainActor
final class SelfHostedService: ObservableObject {

    private let loginManager: LoginManager

    private let userListParams = UserListParams(perPage: 5)
    private let postListParams = PostListParams(perPage: 5)
    private let mediaListParams = MediaListParams(perPage: 5)
    private let termListParams = TermListParams(perPage: 5)

    init(loginManager: LoginManager) {
        self.loginManager = loginManager
    }

    func loadRootListItems() async throws -> [RootListData] {
        let api = try await WordPressAPI.instance(loginManager: self.loginManager)

        var baseData = [
            RootListData(name: "Application Passwords", callback: {
                try await api.applicationPasswords.listWithEditContext(userId: 1)
                    .data
                    .map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Users", sequence: {
                let sequence = api.users.sequenceWithEditContext(params: self.userListParams)
                return ListViewSequence(underlyingSequence: sequence)
            }, category: .system),
            RootListData(name: "Plugins", callback: {
                try await api.plugins.listWithEditContext(params: .init())
                    .data
                    .map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Post Types", callback: {
                try await api.postTypes.listWithEditContext().data.postTypes.map { _, value in
                    value.asListViewData
                }
            }, category: .system),
            RootListData(name: "Media", sequence: {
                let sequence = api.media.sequenceWithEditContext(params: self.mediaListParams)
                return ListViewSequence(underlyingSequence: sequence)
            }, category: .posts),
            RootListData(name: "Site Health Tests", callback: {
                let items: [any ListViewDataConvertable] = [
                    try await api.siteHealthTests.authorizationHeader().data,
                    try await api.siteHealthTests.backgroundUpdates().data,
                    try await api.siteHealthTests.directorySizes().data,
                    try await api.siteHealthTests.dotorgCommunication().data,
                    try await api.siteHealthTests.httpsStatus().data,
                    try await api.siteHealthTests.loopbackRequests().data,
                    try await api.siteHealthTests.pageCache().data
                ]

                return items.map { $0.asListViewData }
            }, category: .system),
            RootListData(name: "Taxonomies", callback: {
                try await api.taxonomies
                    .listWithEditContext(params: TaxonomyListParams())
                    .data
                    .taxonomyTypes
                    .map { (_, value) in
                        value.asListViewData
                    }
            }, category: .system),
            RootListData(name: "Site Settings", callback: {
                return try await api.siteSettings.retrieveWithEditContext().data.asListViewDataItems
            }, category: .system)
        ]

        let postTypes = try await api.postTypes.listWithEditContext().data
            .postTypes
            .map(\.value)
            .filter { $0.visibility.showInNavMenus }
            .filter { $0.supports.map.keys.contains(allOf: [.title, .author, .customFields]) }

        for type in postTypes {
            baseData.append(RootListData(name: type.name, sequence: {
                let sequence = api.posts.sequenceWithEditContext(
                    type: PostEndpointType.custom(type.restBase),
                    params: self.postListParams
                )
                return ListViewSequence(underlyingSequence: sequence)
            }, category: .posts))
        }

        let taxonomyTypes = try await api.taxonomies.listWithEditContext(params: TaxonomyListParams()).data
            .taxonomyTypes
            .map(\.value)
            .filter(\.visibility.showInNavMenus)

        for type in taxonomyTypes {
            baseData.append(RootListData(name: type.name, sequence: {
                let sequence = api.terms.sequenceWithEditContext(
                    type: .custom(type.restBase),
                    params: self.termListParams
                )

                return ListViewSequence(underlyingSequence: sequence)
            }, category: .taxonomies))
        }

        baseData.append(RootListData(name: "Post Statuses", callback: {
            try await api.postStatuses.listWithEditContext()
                .data
                .postStatuses
                .map(\.value.asListViewData)
        }, category: .posts))

        baseData.append(RootListData(name: "Navigations", callback: {
            try await api.navigations.listWithEditContext(params: NavigationListParams())
                .data
                .map(\.asListViewData)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menus", callback: {
            try await api.navMenus.listWithEditContext(params: NavMenuListParams())
                .data
                .map(\.asListViewData)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menu Items", sequence: {
            let sequence = api.navMenuItems
                .sequenceWithEditContext(params: NavMenuItemListParams())

            return ListViewSequence(underlyingSequence: sequence)
        }, category: .navigation))

        baseData.append(RootListData(name: "Menu Locations", callback: {
            try await api.menuLocations.listWithEditContext()
                .data
                .locations
                .map(\.value.asListViewData)
        }, category: .navigation))

        return baseData
    }

}
